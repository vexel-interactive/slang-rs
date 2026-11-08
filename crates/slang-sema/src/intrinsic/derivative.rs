use crate::{Scalar, Type};

pub(crate) fn name(value: &str) -> Option<&'static str> {
    Some(match value {
        "ddx" => "ddx",
        "ddy" => "ddy",
        "ddx_coarse" => "ddx_coarse",
        "ddy_coarse" => "ddy_coarse",
        "ddx_fine" => "ddx_fine",
        "ddy_fine" => "ddy_fine",
        "fwidth" => "fwidth",
        "fwidth_coarse" => "fwidth_coarse",
        "fwidth_fine" => "fwidth_fine",
        _ => return None,
    })
}

/// Integer operands select the corresponding float shape, as in the core module.
pub(crate) fn signature(value: &str, types: &[Type]) -> Option<Type> {
    name(value)?;
    let [ty] = types else { return None };
    Some(match ty.scalar()? {
        Scalar::Int | Scalar::Uint | Scalar::Int64 | Scalar::Uint64 => {
            ty.with_scalar(Scalar::Float)
        }
        Scalar::Half | Scalar::Float | Scalar::Double => ty.clone(),
        Scalar::Bool => return None,
    })
}
