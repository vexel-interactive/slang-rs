//! Contextual scalar initialization for raw byte-buffer atomic inputs.

use crate::{Initialization, InitializationKind, Scalar, Type, checker::Checker};
use slang_source::Diagnostic;
use slang_syntax::{Expr, ExprKind};

impl Checker {
    /// Return whether the argument was a list, including diagnosed invalid lists.
    pub(crate) fn coerce_byte_atomic_list(&mut self, value: &Expr) -> bool {
        if !matches!(value.kind, ExprKind::InitializerList(_)) {
            return false;
        }

        let plan = Planner {
            checker: self,
        }
        .list(value, 0);
        match plan {
            Ok(plan) => {
                self.record_plan_constraints(&plan);
                self.initializations.insert(value.id, plan);
                self.expression_types.insert(value.id, Type::Scalar(Scalar::Uint));
            }
            Err(diagnostic) => self.diagnostics.push(diagnostic),
        }
        true
    }
}

struct Planner<'a> {
    checker: &'a Checker,
}

impl Planner<'_> {
    fn list(&self, value: &Expr, depth: usize) -> Result<Initialization, Diagnostic> {
        if depth >= 32 {
            return Err(Diagnostic::error(
                "T020",
                value.span,
                "aggregate initialization limit exceeded",
            ));
        }
        let uint = Type::Scalar(Scalar::Uint);
        if let ExprKind::InitializerList(values) = &value.kind {
            return match values.as_slice() {
                [] => Ok(Initialization {
                    ty: uint,
                    kind: InitializationKind::Zero,
                }),
                [value] => self.list(value, depth + 1),
                _ => Err(Diagnostic::error("T021", value.span, "too many initializers for uint")),
            };
        }

        let actual = self.checker.expression_types.get(&value.id).unwrap_or(&Type::Error);
        if !matches!(actual.value_type(), Type::Scalar(_) | Type::Vector(_, 1) | Type::Error) {
            return Err(Diagnostic::error(
                "T008",
                value.span,
                format!("cannot initialize uint with {actual}"),
            ));
        }

        // Keep the original expression and its type. Value lowering performs
        // any ConstantBuffer load before the explicit scalar uint conversion.
        Ok(Initialization {
            ty: uint,
            kind: InitializationKind::Value(Box::new(value.clone())),
        })
    }
}
