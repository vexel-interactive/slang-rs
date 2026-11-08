use super::*;
use crate::effects::{assign, expression, variable};

pub(super) fn output(parameter: &Variable) -> bool {
    parameter
        .modifiers
        .iter()
        .any(|m| matches!(m.as_str(), "out" | "inout"))
}

impl Lowerer {
    pub(super) fn argument(
        &mut self,
        parameter: &Variable,
        argument: &Expr,
        statements: &mut Vec<Stmt>,
        fixups: &mut Vec<Stmt>,
        span: Span,
    ) -> Result<Expr, Diagnostic> {
        if !output(parameter) {
            let local = self.storage(parameter, span);
            statements.push(expression(assign(local.clone(), argument.clone())));
            return Ok(local);
        }
        // Coordinates and receivers are evaluated at this argument's position.
        // A simple borrowed address remains aliased throughout the body.
        // A property/swizzle uses the upstream temporary plus fixup. An
        // implicit integer address view asks for aggressive resource addressing.
        let addressable = addressable(argument, parameter.ty != argument.ty);
        let location = self.freeze(argument, statements)?;
        if addressable {
            return Ok(if parameter.ty == argument.ty {
                location
            } else {
                Expr {
                    ty: parameter.ty.clone(),
                    span,
                    kind: ExprKind::AddressView(Box::new(location)),
                }
            });
        }
        let local = self.storage(parameter, span);
        let input = parameter
            .modifiers
            .iter()
            .any(|m| matches!(m.as_str(), "in" | "inout"));
        if input {
            statements.push(expression(assign(local.clone(), location.clone())));
        }
        fixups.push(expression(assign(location, local.clone())));
        Ok(local)
    }

    fn freeze(&mut self, value: &Expr, statements: &mut Vec<Stmt>) -> Result<Expr, Diagnostic> {
        let mut value = value.clone();
        match &mut value.kind {
            ExprKind::AddressView(location) => **location = self.freeze(location, statements)?,
            ExprKind::Name(_) | ExprKind::GlobalName(_) => {}
            ExprKind::Member { base, .. } => **base = self.freeze(base, statements)?,
            ExprKind::Index { base, index } => {
                if matches!(base.ty, Type::Buffer { .. } | Type::Texture { .. }) {
                    **base = self.coordinate(base, statements);
                } else {
                    **base = self.freeze(base, statements)?;
                }
                **index = self.coordinate(index, statements);
            }
            _ => {
                return Err(unsupported(
                    value.span,
                    "ForceInline output argument is not a supported writable location",
                ));
            }
        }
        Ok(scalar_location(value))
    }

    fn coordinate(&mut self, value: &Expr, statements: &mut Vec<Stmt>) -> Expr {
        if matches!(value.kind, ExprKind::Integer(_) | ExprKind::Bool(_)) {
            return value.clone();
        }
        let local = self.storage(
            &variable(String::new(), value.ty.clone(), value.span),
            value.span,
        );
        statements.push(expression(assign(local.clone(), value.clone())));
        local
    }
}

/// HLSL may materialize an indexed vector swizzle as a value. Resolve selected
/// scalar locations to the original storage after their coordinates are frozen.
fn scalar_location(value: Expr) -> Expr {
    match &value.kind {
        ExprKind::Index { base, index }
            if matches!(base.kind, ExprKind::Member { .. })
                && matches!(value.ty, Type::Scalar(_)) =>
        {
            component(*base.clone(), *index.clone())
        }
        ExprKind::Member { base, name } if matches!(value.ty, Type::Scalar(_)) => {
            if let Some(indices) = base.ty.component_indices(name) {
                component(
                    *base.clone(),
                    integer(
                        indices[0],
                        Type::Scalar(slang_sema::Scalar::Int),
                        value.span,
                    ),
                )
            } else {
                value
            }
        }
        _ => value,
    }
}

fn component(base: Expr, index: Expr) -> Expr {
    if let ExprKind::Member { base: source, name } = &base.kind
        && let Some(indices) = source.ty.component_indices(name)
    {
        return component(*source.clone(), selection(&index, &indices));
    }
    match base.ty {
        Type::Matrix(scalar, _, columns) => {
            let row = crate::effects::binary(
                "/",
                index.clone(),
                integer(columns, index.ty.clone(), index.span),
            );
            let column = crate::effects::binary(
                "%",
                index.clone(),
                integer(columns, index.ty.clone(), index.span),
            );
            let row = indexed(base, row, Type::Vector(scalar, columns));
            indexed(row, column, Type::Scalar(scalar))
        }
        Type::Vector(scalar, _) => indexed(base, index, Type::Scalar(scalar)),
        _ => base,
    }
}

fn selection(index: &Expr, components: &[u8]) -> Expr {
    if let ExprKind::Integer(value) = &index.kind
        && let Ok(value) = value.parse::<usize>()
        && let Some(component) = components.get(value)
    {
        return integer(*component, index.ty.clone(), index.span);
    }
    let mut value = integer(*components.last().unwrap(), index.ty.clone(), index.span);
    for (position, component) in components.iter().enumerate().rev().skip(1) {
        let mut condition = crate::effects::binary(
            "==",
            index.clone(),
            integer(position as u8, index.ty.clone(), index.span),
        );
        condition.ty = Type::Scalar(slang_sema::Scalar::Bool);
        value = Expr {
            ty: index.ty.clone(),
            span: index.span,
            kind: ExprKind::Conditional {
                condition: Box::new(condition),
                then_value: Box::new(integer(*component, index.ty.clone(), index.span)),
                else_value: Box::new(value),
            },
        };
    }
    value
}

fn indexed(base: Expr, index: Expr, ty: Type) -> Expr {
    Expr {
        ty,
        span: base.span,
        kind: ExprKind::Index {
            base: Box::new(base),
            index: Box::new(index),
        },
    }
}

fn integer(value: u8, ty: Type, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Integer(value.to_string()),
        ty,
        span,
    }
}

/// Match the supported subset of upstream tryGetAddress(Default). Resource
/// subscript setters use a temporary; a member of that element asks for an
/// aggressive address. Multi-component swizzles have no contiguous address.
fn addressable(value: &Expr, aggressive: bool) -> bool {
    match &value.kind {
        ExprKind::AddressView(location) => addressable(location, aggressive),
        ExprKind::Name(_) | ExprKind::GlobalName(_) => true,
        ExprKind::Member { base, .. } => {
            let scalar_selection = matches!(value.ty, Type::Scalar(_));
            if matches!(base.ty, Type::Vector(..) | Type::Matrix(..)) {
                scalar_selection && addressable(swizzle_base(base), true)
            } else {
                addressable(base, true)
            }
        }
        ExprKind::Index { base, .. } => {
            if matches!(value.ty, Type::Scalar(_))
                && matches!(base.ty, Type::Vector(..))
                && matches!(base.kind, ExprKind::Member { .. })
            {
                return addressable(swizzle_base(base), true);
            }
            if matches!(base.ty, Type::Buffer { .. } | Type::Texture { .. }) {
                aggressive
            } else {
                addressable(base, aggressive)
            }
        }
        _ => false,
    }
}

fn swizzle_base(value: &Expr) -> &Expr {
    if let ExprKind::Member { base, .. } = &value.kind
        && matches!(base.ty, Type::Vector(..) | Type::Matrix(..))
    {
        swizzle_base(base)
    } else {
        value
    }
}
