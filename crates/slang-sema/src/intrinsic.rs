use crate::{Scalar, Type};

mod output;
pub(crate) use output::signature as output_signature;

pub(crate) fn intrinsic(name: &str, types: &[Type]) -> Option<Type> {
    match (name, types) {
        ("mul", [a, b]) if a.numeric() && b.numeric() => product(a, b),
        ("transpose", [Type::Matrix(scalar, rows, columns)]) => {
            Some(Type::Matrix(*scalar, *columns, *rows))
        }
        ("determinant", [Type::Matrix(scalar, rows, columns)])
            if rows == columns
                && matches!(scalar, Scalar::Half | Scalar::Float | Scalar::Double) =>
        {
            Some(Type::Scalar(*scalar))
        }
        ("abs", [a]) if a.numeric() => Some(a.clone()),
        ("clip", [a]) if a.numeric() => Some(Type::Void),
        (
            "sin" | "cos" | "tan" | "sqrt" | "rsqrt" | "floor" | "ceil" | "frac" | "exp" | "exp2"
            | "log" | "log2" | "saturate",
            [a],
        ) if matches!(
            a.scalar(),
            Some(Scalar::Half | Scalar::Float | Scalar::Double)
        ) =>
        {
            Some(a.clone())
        }
        ("min" | "max" | "pow", [a, b]) if a.numeric() && b.numeric() => Type::common(a, b),
        ("clamp" | "lerp", [a, b, c]) if a.numeric() && b.numeric() && c.numeric() => {
            Type::common(&Type::common(a, b)?, c)
        }
        ("dot", [a, b])
            if a == b
                && matches!(
                    a,
                    Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
                ) =>
        {
            Some(Type::Scalar(a.scalar()?))
        }
        ("length", [a])
            if matches!(
                a,
                Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
            ) =>
        {
            Some(Type::Scalar(a.scalar()?))
        }
        ("normalize", [a])
            if matches!(
                a,
                Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
            ) =>
        {
            Some(a.clone())
        }
        ("all" | "any", [a]) if a.scalar().is_some() => Some(Type::Scalar(Scalar::Bool)),
        (
            "GroupMemoryBarrierWithGroupSync"
            | "DeviceMemoryBarrierWithGroupSync"
            | "AllMemoryBarrierWithGroupSync",
            [],
        ) => Some(Type::Void),
        _ => None,
    }
}

/// Algebraic multiplication: matrix dimensions contract rather than broadcast.
fn product(a: &Type, b: &Type) -> Option<Type> {
    let scalar = a.scalar()?.max(b.scalar()?);
    match (a, b) {
        (Type::Scalar(_), _) => Some(b.with_scalar(scalar)),
        (_, Type::Scalar(_)) => Some(a.with_scalar(scalar)),
        (Type::Vector(_, a), Type::Vector(_, b)) if a == b => Some(Type::Scalar(scalar)),
        (Type::Matrix(_, rows, columns), Type::Vector(_, width)) if columns == width => {
            Some(Type::Vector(scalar, *rows))
        }
        (Type::Vector(_, width), Type::Matrix(_, rows, columns)) if width == rows => {
            Some(Type::Vector(scalar, *columns))
        }
        (Type::Matrix(_, rows, inner), Type::Matrix(_, other_inner, columns))
            if inner == other_inner =>
        {
            Some(Type::Matrix(scalar, *rows, *columns))
        }
        _ => None,
    }
}
