use crate::{Scalar, Type, checker::Checker};
use slang_syntax::{Expr, ExprKind};

pub(crate) struct Signature {
    pub parameters: Vec<Type>,
    pub output: Option<usize>,
}

/// The destination fixes the integer type; output arguments cannot convert it.
pub(crate) fn signature(name: &str, types: &[Type]) -> Option<Signature> {
    let output = match name {
        "InterlockedAdd"
        | "InterlockedAnd"
        | "InterlockedOr"
        | "InterlockedXor"
        | "InterlockedMin"
        | "InterlockedMax"
        | "InterlockedExchange"
            if (2..=3).contains(&types.len()) =>
        {
            (types.len() == 3).then_some(2)
        }
        "InterlockedCompareExchange" if types.len() == 4 => Some(3),
        "InterlockedCompareStore" if types.len() == 3 => None,
        _ => return None,
    };
    let ty @ Type::Scalar(Scalar::Int | Scalar::Uint) = types.first()? else {
        return None;
    };
    Some(Signature {
        parameters: vec![ty.clone(); types.len()],
        output,
    })
}

impl Checker {
    pub(crate) fn require_atomic_destination(&mut self, value: &Expr) {
        if !self.atomic_destination(value) {
            self.error(
                "T015",
                value.span,
                "atomic destination requires groupshared storage or an element of an RW resource",
            );
        }
        self.require_writable(value);
    }

    fn atomic_destination(&self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Name(name) => self.lookup(name).is_some_and(|binding| binding.groupshared),
            ExprKind::Member { base, .. } => self.atomic_destination(base),
            ExprKind::Index { base, .. } => {
                if matches!(
                    self.expression_types.get(&base.id),
                    Some(
                        Type::Buffer { writable: true, .. } | Type::Texture { writable: true, .. }
                    )
                ) {
                    true
                } else {
                    self.atomic_destination(base)
                }
            }
            _ => false,
        }
    }
}
