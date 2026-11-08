use super::*;
use slang_sema::{Scalar, Type};

impl Graph {
    /// Blocks using each value, including branch operands and edge arguments.
    pub fn use_blocks(&self) -> Vec<Vec<BlockId>> {
        let mut users = vec![vec![]; self.values.len()];
        for (index, block) in self.blocks.iter().enumerate() {
            let id = BlockId(index);
            for value in block
                .instructions
                .iter()
                .flat_map(|i| i.operation.operands())
                .chain(block.terminator.operands())
            {
                if let Some(list) = users.get_mut(value.0)
                    && list.last() != Some(&id)
                {
                    list.push(id);
                }
            }
        }
        users
    }

    /// Check definitions, operand IDs, edge arity/types, SSA dominance, and
    /// same-block use ordering. Unreachable blocks still receive structural
    /// checks; cross-block dominance is only meaningful on reachable paths.
    pub fn verify(&self) -> Result<(), String> {
        if self.blocks.is_empty() {
            return Err("SSA graph has no entry block".into());
        }
        let mut defined = vec![false; self.values.len()];
        let mut predecessors = vec![vec![]; self.blocks.len()];
        for (index, block) in self.blocks.iter().enumerate() {
            let id = BlockId(index);
            for (index, &value) in block.parameters.iter().enumerate() {
                self.definition(
                    value,
                    Definition::Parameter { block: id, index },
                    &mut defined,
                )?;
            }
            for (index, instruction) in block.instructions.iter().enumerate() {
                self.definition(
                    instruction.result,
                    Definition::Instruction { block: id, index },
                    &mut defined,
                )?;
                for operand in instruction.operation.operands() {
                    self.value(operand)?;
                }
                if let Operation::Copy(operand) = instruction.operation
                    && self.values[operand.0].ty != self.values[instruction.result.0].ty
                {
                    return Err("SSA copy changes its operand type".into());
                }
            }
            for operand in block.terminator.operands() {
                self.value(operand)?;
            }
            if let Terminator::Branch { condition, .. } = block.terminator
                && self.values[condition.0].ty != Type::Scalar(Scalar::Bool)
            {
                return Err("SSA branch condition is not boolean".into());
            }
            if let Terminator::Return(Some(value)) = block.terminator
                && self.values[value.0].ty != self.return_type
            {
                return Err("SSA return value has the wrong type".into());
            }
            for edge in block.terminator.edges() {
                let Some(target) = self.blocks.get(edge.target.0) else {
                    return Err("SSA edge targets a missing block".into());
                };
                if edge.arguments.len() != target.parameters.len() {
                    return Err("SSA edge argument count does not match block parameters".into());
                }
                for (&argument, &parameter) in edge.arguments.iter().zip(&target.parameters) {
                    self.value(parameter)?;
                    if self.values[argument.0].ty != self.values[parameter.0].ty {
                        return Err("SSA edge argument has the wrong type".into());
                    }
                }
                predecessors[edge.target.0].push(id);
            }
        }
        if defined.iter().any(|&value| !value) {
            return Err("SSA value has no definition".into());
        }
        let (dominators, reachable) = self.dominators(&predecessors);
        for (index, block) in self.blocks.iter().enumerate() {
            let id = BlockId(index);
            for (index, instruction) in block.instructions.iter().enumerate() {
                for operand in instruction.operation.operands() {
                    self.use_value(operand, id, index, &dominators, &reachable)?;
                }
            }
            for operand in block.terminator.operands() {
                self.use_value(
                    operand,
                    id,
                    block.instructions.len(),
                    &dominators,
                    &reachable,
                )?;
            }
        }
        Ok(())
    }

    fn value(&self, value: ValueId) -> Result<&Value, String> {
        self.values
            .get(value.0)
            .ok_or_else(|| "SSA operand references a missing value".into())
    }

    fn definition(
        &self,
        value: ValueId,
        expected: Definition,
        defined: &mut [bool],
    ) -> Result<(), String> {
        if self.value(value)?.definition != expected {
            return Err("SSA definition does not match its owning block".into());
        }
        if std::mem::replace(&mut defined[value.0], true) {
            return Err("SSA value is defined more than once".into());
        }
        Ok(())
    }

    fn use_value(
        &self,
        value: ValueId,
        block: BlockId,
        index: usize,
        dominators: &[Option<BlockId>],
        reachable: &[bool],
    ) -> Result<(), String> {
        let (owner, instruction) = match self.values[value.0].definition {
            Definition::Parameter { block, .. } => (block, None),
            Definition::Instruction { block, index } => (block, Some(index)),
        };
        if owner == block {
            if instruction.is_some_and(|definition| definition >= index) {
                return Err("SSA value is used before its definition".into());
            }
        } else if reachable[block.0] {
            let mut ancestor = block;
            while ancestor != owner {
                let Some(parent) = dominators[ancestor.0] else {
                    return Err("SSA definition does not dominate its use".into());
                };
                if parent == ancestor {
                    return Err("SSA definition does not dominate its use".into());
                }
                ancestor = parent;
            }
        }
        Ok(())
    }

    /// Cooper's immediate-dominator iteration in reverse postorder avoids
    /// allocating a quadratic matrix of dominator sets for large CFGs.
    fn dominators(&self, predecessors: &[Vec<BlockId>]) -> (Vec<Option<BlockId>>, Vec<bool>) {
        let mut reachable = vec![false; self.blocks.len()];
        let mut postorder = vec![];
        let mut stack = vec![(BlockId(0), false)];
        while let Some((block, finish)) = stack.pop() {
            if finish {
                postorder.push(block);
                continue;
            }
            if std::mem::replace(&mut reachable[block.0], true) {
                continue;
            }
            stack.push((block, true));
            for edge in self.blocks[block.0].terminator.edges().into_iter().rev() {
                if !reachable[edge.target.0] {
                    stack.push((edge.target, false));
                }
            }
        }
        postorder.reverse();
        let mut order = vec![0; self.blocks.len()];
        for (index, block) in postorder.iter().enumerate() {
            order[block.0] = index;
        }
        let mut parents = vec![None; self.blocks.len()];
        parents[0] = Some(BlockId(0));
        loop {
            let mut changed = false;
            for &block in postorder.iter().skip(1) {
                let mut incoming = predecessors[block.0]
                    .iter()
                    .copied()
                    .filter(|p| parents[p.0].is_some());
                let Some(mut parent) = incoming.next() else {
                    continue;
                };
                for mut other in incoming {
                    while parent != other {
                        while order[parent.0] > order[other.0] {
                            parent = parents[parent.0].unwrap();
                        }
                        while order[other.0] > order[parent.0] {
                            other = parents[other.0].unwrap();
                        }
                    }
                }
                if parents[block.0] != Some(parent) {
                    parents[block.0] = Some(parent);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        (parents, reachable)
    }
}
