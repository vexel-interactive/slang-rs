use super::Signature;
use crate::{Scalar, Type};

/// Integer core functions preserve width and use uint for counts and positions.
pub(crate) fn signature(name: &str, types: &[Type]) -> Option<Signature> {
    if !matches!(
        name,
        "countbits" | "reversebits" | "firstbithigh" | "firstbitlow"
    ) {
        return None;
    }
    let [input] = types else { return None };
    if !matches!(input, Type::Scalar(_) | Type::Vector(..)) {
        return None;
    }
    let parameter = match input.scalar()? {
        Scalar::Bool => input.with_scalar(Scalar::Int),
        Scalar::Int | Scalar::Uint => input.clone(),
        _ => return None,
    };
    let result = if name == "reversebits" {
        parameter.clone()
    } else {
        parameter.with_scalar(Scalar::Uint)
    };
    Some(Signature {
        parameters: vec![parameter],
        result,
        outputs_start: None,
    })
}
