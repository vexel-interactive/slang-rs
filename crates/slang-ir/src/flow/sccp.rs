use super::{constant, *};
use serde::Serialize;
use std::collections::VecDeque;

/// SCCP's three-point lattice. `None` means no executable definition yet;
/// `Any` means a dynamic value, including uninitialized or opaque storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Lattice {
    None,
    Constant(i64),
    Any,
}

impl Lattice {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, value) | (value, Self::None) => value,
            (Self::Constant(a), Self::Constant(b)) if a == b => Self::Constant(a),
            _ => Self::Any,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Propagation {
    pub values: Vec<Lattice>,
    pub reachable: Vec<bool>,
    /// Edge order follows Terminator::edges(). Edges become executable
    /// monotonically; a dynamic condition activates all of its successors.
    pub executable_edges: Vec<Vec<bool>>,
}

/// Propagate integer and boolean constants through a verified SSA graph.
/// Use-def worklists revisit only executable blocks affected by a value or
/// edge change. Block arguments meet values from executable predecessors.
pub fn propagate(graph: &Graph) -> Propagation {
    let mut solver = Solver {
        graph,
        result: Propagation {
            values: vec![Lattice::None; graph.values.len()],
            reachable: vec![false; graph.blocks.len()],
            executable_edges: graph
                .blocks
                .iter()
                .map(|b| vec![false; b.terminator.edges().len()])
                .collect(),
        },
        users: graph.use_blocks(),
        pending: VecDeque::new(),
        queued: vec![false; graph.blocks.len()],
    };
    if !graph.blocks.is_empty() {
        solver.result.reachable[0] = true;
        for parameter in &graph.blocks[0].parameters {
            solver.result.values[parameter.0] = Lattice::Any;
        }
        solver.enqueue(BlockId(0));
    }
    while let Some(block) = solver.pending.pop_front() {
        solver.queued[block.0] = false;
        solver.block(block);
    }
    solver.result
}

struct Solver<'a> {
    graph: &'a Graph,
    result: Propagation,
    users: Vec<Vec<BlockId>>,
    pending: VecDeque<BlockId>,
    queued: Vec<bool>,
}

impl Solver<'_> {
    fn enqueue(&mut self, block: BlockId) {
        if self.result.reachable[block.0] && !self.queued[block.0] {
            self.queued[block.0] = true;
            self.pending.push_back(block);
        }
    }

    fn update(&mut self, value: ValueId, incoming: Lattice) {
        let old = self.result.values[value.0];
        let new = old.join(incoming);
        if old != new {
            self.result.values[value.0] = new;
            for block in self.users[value.0].clone() {
                self.enqueue(block);
            }
        }
    }

    fn block(&mut self, id: BlockId) {
        let block = &self.graph.blocks[id.0];
        for instruction in &block.instructions {
            let value = self.evaluate(instruction);
            self.update(instruction.result, value);
        }
        let edges = block.terminator.edges();
        let active: Vec<usize> = match &block.terminator {
            Terminator::Jump(_) => vec![0],
            Terminator::Branch { condition, .. } => match self.result.values[condition.0] {
                Lattice::None => vec![],
                Lattice::Constant(bits) => vec![usize::from(bits == 0)],
                Lattice::Any => vec![0, 1],
            },
            Terminator::Switch {
                selector, cases, ..
            } => match self.result.values[selector.0] {
                Lattice::None => vec![],
                Lattice::Constant(bits) => vec![
                    cases
                        .iter()
                        .position(|(label, _)| *label == bits)
                        .unwrap_or(cases.len()),
                ],
                Lattice::Any => (0..edges.len()).collect(),
            },
            _ => vec![],
        };
        for index in active {
            let edge = edges[index];
            self.result.executable_edges[id.0][index] = true;
            let fresh = !self.result.reachable[edge.target.0];
            self.result.reachable[edge.target.0] = true;
            for (&argument, &parameter) in edge
                .arguments
                .iter()
                .zip(&self.graph.blocks[edge.target.0].parameters)
            {
                self.update(parameter, self.result.values[argument.0]);
            }
            if fresh {
                self.enqueue(edge.target);
            }
        }
    }

    fn evaluate(&self, instruction: &Instruction) -> Lattice {
        let Some(target) = constant::scalar(&self.graph.values[instruction.result.0].ty) else {
            return Lattice::Any;
        };
        let unary = |value: ValueId, operation: &dyn Fn(i64) -> Option<i64>| match self
            .result
            .values[value.0]
        {
            Lattice::Constant(bits) => operation(bits).map_or(Lattice::Any, Lattice::Constant),
            other => other,
        };
        match &instruction.operation {
            Operation::Constant(bits) => Lattice::Constant(constant::normalize(*bits, target)),
            Operation::Unknown(_) => Lattice::Any,
            Operation::Copy(value) => self.result.values[value.0],
            Operation::Convert(value)
                if constant::scalar(&self.graph.values[value.0].ty).is_some() =>
            {
                unary(*value, &|bits| Some(constant::normalize(bits, target)))
            }
            Operation::Unary { op, value } => {
                unary(*value, &|bits| constant::unary(op, bits, target))
            }
            Operation::Binary { op, left, right } => {
                let operand = constant::scalar(&self.graph.values[left.0].ty);
                match (self.result.values[left.0], self.result.values[right.0]) {
                    (Lattice::Any, _) | (_, Lattice::Any) => Lattice::Any,
                    (Lattice::None, _) | (_, Lattice::None) => Lattice::None,
                    (Lattice::Constant(a), Lattice::Constant(b)) => operand
                        .and_then(|operand| constant::binary(op, a, b, operand, target))
                        .map_or(Lattice::Any, Lattice::Constant),
                }
            }
            _ => Lattice::Any,
        }
    }
}
