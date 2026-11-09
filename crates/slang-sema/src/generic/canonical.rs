use super::*;
use crate::Type;

impl Specializer {
    pub(super) fn canonical(&mut self, ty: &mut TypeRef, depth: usize) -> Result<(), Diagnostic> {
        if depth >= 32 {
            return Err(error(
                ty.span,
                "cyclic or excessively nested generic argument",
            ));
        }
        self.expand(ty)?;
        if let Some(target) = self.aliases.get(&ty.name).cloned() {
            let mut target = target;
            self.canonical(&mut target, depth + 1)?;
            target.arrays.splice(0..0, ty.arrays.clone());
            target.matrix_major = ty.matrix_major.or(target.matrix_major);
            target.span = ty.span;
            *ty = target;
        }
        for argument in &mut ty.arguments {
            if let TypeArgument::Type(argument) = argument {
                self.canonical(argument, depth + 1)?;
            }
        }
        for size in &mut ty.arrays {
            if let Some(value) = self.shape_value(size) {
                size.kind = ExprKind::Integer(value.to_string());
                size.parenthesized = false;
            }
        }
        if let Some(builtin) = Type::builtin(&ty.name) {
            ty.name = builtin.to_string();
        }
        if matches!(ty.name.as_str(), "vector" | "matrix") {
            let scalar = match ty.arguments.first() {
                Some(TypeArgument::Type(component)) if component.arrays.is_empty() => {
                    Type::builtin(&component.name).and_then(|t| t.scalar())
                }
                None => Some(crate::Scalar::Float),
                _ => None,
            };
            let dimensions = ty
                .arguments
                .iter()
                .skip(1)
                .map(|a| match a {
                    TypeArgument::Integer { value, .. } => u8::try_from(*value).ok(),
                    TypeArgument::Expression(e) => {
                        self.shape_value(e).and_then(|n| u8::try_from(n).ok())
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>();
            if let (Some(scalar), Some(mut dimensions)) = (scalar, dimensions) {
                let expected = if ty.name == "vector" { 1 } else { 2 };
                if dimensions.len() <= expected && dimensions.iter().all(|&d| (1..=4).contains(&d))
                {
                    dimensions.resize(expected, 4);
                    ty.name = if expected == 1 {
                        Type::Vector(scalar, dimensions[0]).to_string()
                    } else {
                        Type::Matrix(scalar, dimensions[0], dimensions[1]).to_string()
                    };
                    ty.arguments.clear();
                    ty.application = false;
                }
            }
        }
        Ok(())
    }

    pub(super) fn shape_value(&mut self, expression: &Expr) -> Option<i64> {
        let count = self.shapes.diagnostics.len();
        let value = self.shapes.numeric_shape_value(expression);
        self.shapes.diagnostics.truncate(count);
        value
    }
}

pub(super) fn core_type(name: &str) -> bool {
    Type::builtin(name).is_some()
        || matches!(
            name,
            "vector" | "matrix" | "StructuredBuffer" | "RWStructuredBuffer" | "ConstantBuffer"
        )
        || Type::texture(name, Type::Error).is_some()
}

pub(super) fn type_key(ty: &TypeRef) -> String {
    let mut name = ty.name.clone();
    if let Some(major) = ty.matrix_major {
        name = format!("{major:?} {name}");
    }
    if !ty.arguments.is_empty() {
        name.push_str(&format!(
            "<{}>",
            ty.arguments
                .iter()
                .map(|a| match a {
                    TypeArgument::Type(t) => type_key(t),
                    TypeArgument::Integer { value, .. } => value.to_string(),
                    TypeArgument::Expression(e) => expression_key(e),
                })
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    for size in &ty.arrays {
        name.push_str(&format!("[{}]", expression_key(size)));
    }
    name
}

fn expression_key(expression: &Expr) -> String {
    match &expression.kind {
        ExprKind::Name(n) | ExprKind::Integer(n) | ExprKind::Float(n) | ExprKind::String(n) => {
            n.clone()
        }
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Unary { op, value } => format!("({op}{})", expression_key(value)),
        ExprKind::Binary { op, left, right } => {
            format!("({}{op}{})", expression_key(left), expression_key(right))
        }
        _ => format!("expression_{}", expression.id.0),
    }
}
