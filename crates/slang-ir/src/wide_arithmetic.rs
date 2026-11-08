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
        if let ExprKind::Binary { op, left, .. } = &expression.kind
            && matches!(op.as_str(), "<<" | ">>")
            && matches!(left.ty.scalar(), Some(Scalar::Int16 | Scalar::Uint16))
        {
            // DXC masks native 16-bit shift counts. Pinned SCCP saturates at
            // the operand width: zero, or sign extension for signed right shifts.
            let mut operation = expression.clone();
            let ExprKind::Binary { left, right, .. } = &mut operation.kind else {
                unreachable!()
            };
            let scalar = if left.ty.scalar() == Some(Scalar::Uint16) {
                Scalar::Uint
            } else {
                Scalar::Int
            };
            let promoted = operation.ty.with_scalar(scalar);
            coerce(left, &promoted);
            let count_scalar = match right.ty.scalar().unwrap() {
                Scalar::Int16 | Scalar::Uint16 => Scalar::Uint16,
                Scalar::Int | Scalar::Uint => Scalar::Uint,
                _ => Scalar::Uint64,
            };
            let count_type = right.ty.with_scalar(count_scalar);
            coerce(right, &count_type);
            **right = Expr {
                ty: count_type.clone(),
                span: right.span,
                kind: ExprKind::IntrinsicCall {
                    name: "min".into(),
                    arguments: vec![
                        right.as_ref().clone(),
                        Expr {
                            ty: Type::Scalar(count_scalar),
                            span: right.span,
                            kind: ExprKind::Integer("16u".into()),
                        },
                    ],
                },
            };
            operation.ty = promoted;
            expression.kind = ExprKind::Cast {
                value: Box::new(operation),
            };
            return Ok(());
        }
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
            } if wide(&expression.ty) || wide(&then_value.ty) || wide(&else_value.ty) => {
                coerce(then_value, &expression.ty);
                coerce(else_value, &expression.ty);
            }
            ExprKind::IntrinsicCall { name, arguments }
                if (wide(&expression.ty) || arguments.iter().any(|a| wide(&a.ty)))
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
    matches!(
        ty.scalar(),
        Some(
            Scalar::Int16
                | Scalar::Uint16
                | Scalar::Int64
                | Scalar::Uint64
                | Scalar::IntPtr
                | Scalar::UintPtr
        )
    )
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
