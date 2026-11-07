use crate::{Scalar, Type};

pub(crate) struct Signature {
    pub parameters: Vec<Type>,
    pub result: Type,
}

/// Output arguments participate in type inference but cannot be implicitly cast.
pub(crate) fn signature(name: &str, types: &[Type]) -> Option<Signature> {
    let count = match name {
        "sincos" => 3,
        "modf" | "frexp" => 2,
        _ => return None,
    };
    if types.len() != count {
        return None;
    }
    let mut target = types[0].clone();
    if name == "frexp" {
        let shape = types[1].with_scalar(target.scalar()?);
        target = Type::common(&target, &shape)?;
    } else {
        for ty in &types[1..] {
            target = if target == Type::InitializerList {
                ty.clone()
            } else {
                Type::common(&target, ty)?
            };
        }
    }
    if !target.numeric() {
        return None;
    }
    let scalar = target.scalar()?;
    if matches!(scalar, Scalar::Int | Scalar::Uint) {
        target = target.with_scalar(Scalar::Float);
    }
    let output = if name == "frexp" {
        target.with_scalar(Scalar::Int)
    } else {
        target.clone()
    };
    let result = if name == "sincos" {
        Type::Void
    } else {
        target.clone()
    };
    let mut parameters = vec![target];
    parameters.extend(std::iter::repeat_n(output, count - 1));
    Some(Signature { parameters, result })
}
