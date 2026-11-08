use crate::{Scalar, Type};

mod atomic;
mod bitcast;
mod bitops;
mod derivative;
mod geometric;
mod output;
pub(crate) use atomic::signature as atomic_signature;
pub(crate) use bitcast::signature as bitcast_signature;
pub(crate) use bitops::signature as bitops_signature;
pub(crate) use derivative::{name as derivative_name, signature as derivative_signature};
pub(crate) use geometric::signature as geometric_signature;
pub(crate) use output::signature as output_signature;

pub(crate) struct Signature {
    pub parameters: Vec<Type>,
    pub result: Type,
    pub outputs_start: Option<usize>,
}

/// Names implemented by the native builtin signature set.
pub fn is_intrinsic_name(name: &str) -> bool {
    derivative::name(name).is_some()
        || matches!(
            name,
            "abs"
                | "asint"
                | "asuint"
                | "asfloat"
                | "asdouble"
                | "countbits"
                | "reversebits"
                | "firstbithigh"
                | "firstbitlow"
                | "sin"
                | "cos"
                | "tan"
                | "sqrt"
                | "rsqrt"
                | "floor"
                | "ceil"
                | "frac"
                | "exp"
                | "exp2"
                | "log"
                | "log2"
                | "saturate"
                | "min"
                | "max"
                | "pow"
                | "clamp"
                | "lerp"
                | "dot"
                | "length"
                | "normalize"
                | "cross"
                | "distance"
                | "reflect"
                | "refract"
                | "faceforward"
                | "mul"
                | "transpose"
                | "determinant"
                | "all"
                | "any"
                | "clip"
                | "sincos"
                | "modf"
                | "frexp"
                | "InterlockedAdd"
                | "InterlockedAnd"
                | "InterlockedOr"
                | "InterlockedXor"
                | "InterlockedMin"
                | "InterlockedMax"
                | "InterlockedExchange"
                | "InterlockedCompareExchange"
                | "InterlockedCompareStore"
                | "GroupMemoryBarrierWithGroupSync"
                | "DeviceMemoryBarrierWithGroupSync"
                | "AllMemoryBarrierWithGroupSync"
                | "GroupMemoryBarrier"
                | "DeviceMemoryBarrier"
                | "AllMemoryBarrier"
        )
}

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
        ("min" | "max" | "pow", [a, b]) if a.numeric() && b.numeric() => common_arguments(a, b),
        ("clamp" | "lerp", [a, b, c]) if a.numeric() && b.numeric() && c.numeric() => {
            common_arguments(&common_arguments(a, b)?, c)
        }
        ("all" | "any", [a]) if a.scalar().is_some() => Some(Type::Scalar(Scalar::Bool)),
        (
            "GroupMemoryBarrierWithGroupSync"
            | "DeviceMemoryBarrierWithGroupSync"
            | "AllMemoryBarrierWithGroupSync"
            | "GroupMemoryBarrier"
            | "DeviceMemoryBarrier"
            | "AllMemoryBarrier",
            [],
        ) => Some(Type::Void),
        _ => None,
    }
}

/// Numeric generic parameters join component constraints in argument order.
/// Matrix overloads require matrices on both sides rather than scalar splats.
fn common_arguments(a: &Type, b: &Type) -> Option<Type> {
    if matches!(a, Type::Matrix(..)) != matches!(b, Type::Matrix(..)) {
        return None;
    }
    let shape = Type::common(a, b)?;
    Some(shape.with_scalar(a.scalar()?.inferred_common(b.scalar()?)))
}

/// Algebraic multiplication: matrix dimensions contract rather than broadcast.
fn product(a: &Type, b: &Type) -> Option<Type> {
    let scalar = a.scalar()?.inferred_common(b.scalar()?);
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
