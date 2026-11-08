use super::Signature;
use crate::{Scalar, Type};

/// Resolve the 32-bit overloads and the upstream double packing signatures.
pub(crate) fn signature(
    name: &str,
    types: &[Type],
    literal: Option<slang_lexer::IntegerLiteral>,
) -> Option<Signature> {
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
        Scalar::Double => return None,
        Scalar::Int64 | Scalar::Uint64 | Scalar::IntPtr | Scalar::UintPtr => {
            let mut candidates = [Scalar::Int, Scalar::Uint, Scalar::Float].map(|scalar| {
                let target = input.with_scalar(scalar);
                (input.literal_conversion_cost(&target, literal), target)
            });
            candidates.sort_by_key(|(cost, _)| *cost);
            if candidates[0].0 == candidates[1].0 {
                return None;
            }
            candidates[0].1.clone()
        }
    };
    Some(Signature {
        parameters: vec![parameter],
        result: input.with_scalar(scalar),
        outputs_start: None,
    })
}
