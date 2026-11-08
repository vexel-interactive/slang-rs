use super::Signature;
use crate::{Scalar, Type};

/// Resolve the 32-bit overloads and the upstream double packing signatures.
pub(crate) fn signature(name: &str, types: &[Type]) -> Option<Signature> {
    if name == "asuint"
        && let [input, low, high] = types
        && matches!(input, Type::Scalar(_) | Type::Vector(_, 1))
        && [low, high]
            .iter()
            .all(|ty| matches!(ty, Type::Scalar(Scalar::Int | Scalar::Uint)))
    {
        return Some(Signature {
            parameters: vec![Type::Scalar(Scalar::Double), low.clone(), high.clone()],
            result: Type::Void,
            outputs_start: Some(1),
        });
    }
    if name == "asdouble" {
        let [low, high] = types else { return None };
        let width = low.width()?.max(high.width()?);
        if width > 2 {
            return None;
        }
        let shape = Type::numeric_shape(Scalar::Uint, width);
        return Some(Signature {
            parameters: vec![shape.clone(), shape.clone()],
            result: shape.with_scalar(Scalar::Double),
            outputs_start: None,
        });
    }
    let scalar = match name {
        "asint" => Scalar::Int,
        "asuint" => Scalar::Uint,
        "asfloat" => Scalar::Float,
        _ => return None,
    };
    let [input] = types else { return None };
    let parameter = match input.scalar()? {
        Scalar::Bool => input.with_scalar(Scalar::Int),
        Scalar::Half => input.with_scalar(Scalar::Float),
        Scalar::Int | Scalar::Uint | Scalar::Float => input.clone(),
        Scalar::Double | Scalar::Int64 | Scalar::Uint64 => return None,
    };
    Some(Signature {
        parameters: vec![parameter],
        result: input.with_scalar(scalar),
        outputs_start: None,
    })
}
