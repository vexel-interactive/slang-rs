use crate::checker::Checker;
use slang_syntax::{Expr, ExprKind, Function};
use std::collections::BTreeSet;

impl Checker {
    pub(crate) fn parameter_defaults(&mut self, function: &Function) {
        let names: BTreeSet<_> = function
            .parameters
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        let mut seen = false;
        for parameter in &function.parameters {
            if let Some(value) = &parameter.initializer {
                seen = true;
                if parameter
                    .modifiers
                    .iter()
                    .any(|m| matches!(m.as_str(), "out" | "inout"))
                {
                    self.error(
                        "T022",
                        parameter.span,
                        "output parameters cannot have default arguments",
                    );
                }
                if references_parameter(value, &names) {
                    self.error(
                        "T022",
                        value.span,
                        "parameter references in default arguments are not implemented",
                    );
                }
                let ty = self.resolve_type(&parameter.ty);
                self.check_value(value, &ty);
            } else if seen {
                self.error(
                    "T022",
                    parameter.span,
                    "a required parameter cannot follow a default argument",
                );
            }
        }
    }
}

fn references_parameter(value: &Expr, names: &BTreeSet<&str>) -> bool {
    match &value.kind {
        ExprKind::Name(name) => names.contains(name.as_str()),
        ExprKind::InitializerList(values) => values.iter().any(|v| references_parameter(v, names)),
        ExprKind::Unary { value, .. }
        | ExprKind::Cast { value, .. }
        | ExprKind::Postfix { value, .. } => references_parameter(value, names),
        ExprKind::Binary { left, right, .. } => {
            references_parameter(left, names) || references_parameter(right, names)
        }
        ExprKind::Call { callee, arguments } => {
            references_parameter(callee, names)
                || arguments.iter().any(|v| references_parameter(v, names))
        }
        ExprKind::Member { base, .. } => references_parameter(base, names),
        ExprKind::Index { base, index } => {
            references_parameter(base, names) || references_parameter(index, names)
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            references_parameter(condition, names)
                || references_parameter(then_value, names)
                || references_parameter(else_value, names)
        }
        _ => false,
    }
}
