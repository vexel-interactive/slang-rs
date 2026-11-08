use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::Diagnostic;

/// Preserve checked integer promotions and compound assignment operand types.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    Lowerer.module(module)
}

struct Lowerer;

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        match &mut expression.kind {
            ExprKind::Binary { op, left, right }
                if matches!(
                    op.as_str(),
                    "+" | "-"
                        | "*"
                        | "/"
                        | "%"
                        | "&"
                        | "|"
                        | "^"
                        | "=="
                        | "!="
                        | "<"
                        | "<="
                        | ">"
                        | ">="
                        | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "&="
                        | "|="
                        | "^="
                ) =>
            {
                if op.ends_with('=') && !matches!(op.as_str(), "==" | "!=" | "<=" | ">=") {
                    // Slang's compound overload fixes the operand type from its
                    // writable left argument. A native compound expression also
                    // lets DXC evaluate an indexed destination exactly once.
                    coerce(right, &left.ty);
                } else if let Some(common) = Type::common(&left.ty, &right.ty).filter(wide) {
                    coerce(left, &common);
                    coerce(right, &common);
                }
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } if wide(&expression.ty) => {
                coerce(then_value, &expression.ty);
                coerce(else_value, &expression.ty);
            }
            ExprKind::IntrinsicCall { name, arguments }
                if wide(&expression.ty)
                    && matches!(name.as_str(), "min" | "max" | "clamp" | "mul") =>
            {
                for argument in arguments {
                    let target = if name == "mul" {
                        argument.ty.with_scalar(expression.ty.scalar().unwrap())
                    } else {
                        expression.ty.clone()
                    };
                    coerce(argument, &target);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn wide(ty: &Type) -> bool {
    matches!(ty.scalar(), Some(Scalar::Int64 | Scalar::Uint64))
}

fn coerce(expression: &mut Expr, ty: &Type) {
    if expression.ty != *ty {
        *expression = Expr {
            ty: ty.clone(),
            span: expression.span,
            kind: ExprKind::Cast {
                value: Box::new(expression.clone()),
            },
        };
    }
}
