use super::Signature;
use crate::{Scalar, Type};

/// Geometric overloads infer a common component type and retain vector dimensions.
pub(crate) fn signature(name: &str, types: &[Type]) -> Option<Signature> {
    let arity = match name {
        "length" | "normalize" => 1,
        "cross" | "distance" | "reflect" | "dot" => 2,
        "refract" | "faceforward" => 3,
        _ => return None,
    };
    if types.len() != arity || types.iter().any(|ty| ty.width().is_none()) {
        return None;
    }
    if name == "cross" {
        return cross(types);
    }
    if name != "dot" && types.iter().any(|ty| ty.scalar() == Some(Scalar::Bool)) {
        return None;
    }
    if name == "refract" && types[2].width() != Some(1) {
        return None;
    }
    let inputs = if name == "refract" {
        &types[..2]
    } else {
        types
    };
    let mut width = None;
    for ty in inputs {
        if let Type::Vector(_, n) = ty {
            if width.is_some_and(|w| w != *n) {
                return None;
            }
            width = Some(*n);
        }
    }
    let scalar = if name == "dot" {
        arithmetic_component(types)?
    } else {
        match types
            .iter()
            .filter_map(Type::scalar)
            .reduce(Scalar::inferred_common)?
        {
            Scalar::Int
            | Scalar::Uint
            | Scalar::Int64
            | Scalar::Uint64
            | Scalar::IntPtr
            | Scalar::UintPtr => Scalar::Float,
            scalar => scalar,
        }
    };
    let parameter = match width {
        Some(n) => Type::Vector(scalar, n),
        None => Type::Scalar(scalar),
    };
    let mut parameters = vec![parameter.clone(); arity];
    if name == "refract" {
        parameters[2] = Type::Scalar(scalar);
    }
    let result = match name {
        "length" | "distance" | "dot" => Type::Scalar(scalar),
        // The core faceforward signature is always a vector, even for scalar inputs.
        "faceforward" => Type::Vector(scalar, width.unwrap_or(1)),
        _ => parameter,
    };
    Some(Signature {
        parameters,
        result,
        outputs_start: None,
    })
}

fn cross(types: &[Type]) -> Option<Signature> {
    if types
        .iter()
        .any(|ty| !matches!(ty.width(), Some(1 | 3)) || matches!(ty, Type::Vector(Scalar::Bool, 1)))
    {
        return None;
    }
    let result = Type::Vector(arithmetic_component(types)?, 3);
    Some(Signature {
        parameters: vec![result.clone(); 2],
        result,
        outputs_start: None,
    })
}

/// Bool reaches an integer overload, so floating operands convert to that integer type.
fn arithmetic_component(types: &[Type]) -> Option<Scalar> {
    let scalar = if types.iter().any(|ty| ty.scalar() == Some(Scalar::Bool)) {
        if types.iter().any(|ty| ty.scalar() == Some(Scalar::Uint)) {
            Scalar::Uint
        } else {
            Scalar::Int
        }
    } else {
        types
            .iter()
            .filter_map(Type::scalar)
            .reduce(Scalar::inferred_common)?
    };
    Some(scalar)
}
