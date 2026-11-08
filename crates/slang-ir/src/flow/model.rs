use serde::Serialize;
use slang_sema::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ValueId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct BlockId(pub usize);

/// An analysis CFG in SSA form, with typed values and explicit block arguments.
#[derive(Clone, Debug, Serialize)]
pub struct Graph {
    pub blocks: Vec<Block>,
    pub values: Vec<Value>,
    pub return_type: Type,
}

#[derive(Clone, Debug, Serialize)]
pub struct Value {
    pub ty: Type,
    pub definition: Definition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Definition {
    Parameter { block: BlockId, index: usize },
    Instruction { block: BlockId, index: usize },
}

#[derive(Clone, Debug, Serialize)]
pub struct Block {
    pub parameters: Vec<ValueId>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Clone, Debug, Serialize)]
pub struct Instruction {
    pub result: ValueId,
    pub operation: Operation,
}

#[derive(Clone, Debug, Serialize)]
pub enum Operation {
    Constant(i64),
    /// Unknown input, uninitialized storage, opaque load, or call result.
    /// Operands retain use-def information and source evaluation dependencies.
    Unknown(Vec<ValueId>),
    Copy(ValueId),
    Convert(ValueId),
    Unary {
        op: String,
        value: ValueId,
    },
    Binary {
        op: String,
        left: ValueId,
        right: ValueId,
    },
}

impl Operation {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Constant(_) => vec![],
            Self::Unknown(values) => values.clone(),
            Self::Copy(value) | Self::Convert(value) | Self::Unary { value, .. } => vec![*value],
            Self::Binary { left, right, .. } => vec![*left, *right],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Edge {
    pub target: BlockId,
    pub arguments: Vec<ValueId>,
}

#[derive(Clone, Debug, Serialize)]
pub enum Terminator {
    Jump(Edge),
    Branch {
        condition: ValueId,
        on_true: Edge,
        on_false: Edge,
    },
    Switch {
        selector: ValueId,
        cases: Vec<(i64, Edge)>,
        default: Edge,
    },
    Return(Option<ValueId>),
    Unreachable,
}

impl Terminator {
    pub fn edges(&self) -> Vec<&Edge> {
        match self {
            Self::Jump(edge) => vec![edge],
            Self::Branch {
                on_true, on_false, ..
            } => vec![on_true, on_false],
            Self::Switch { cases, default, .. } => cases
                .iter()
                .map(|(_, edge)| edge)
                .chain([default])
                .collect(),
            _ => vec![],
        }
    }

    pub fn operands(&self) -> Vec<ValueId> {
        let mut result = match self {
            Self::Branch { condition, .. } => vec![*condition],
            Self::Switch { selector, .. } => vec![*selector],
            Self::Return(Some(value)) => vec![*value],
            _ => vec![],
        };
        result.extend(
            self.edges()
                .into_iter()
                .flat_map(|edge| &edge.arguments)
                .copied(),
        );
        result
    }
}
