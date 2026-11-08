use slang_sema::{Scalar, Type};

pub(super) fn scalar(ty: &Type) -> Option<Scalar> {
    match ty {
        Type::Scalar(s) if s.is_integral() || *s == Scalar::Bool => Some(*s),
        _ => None,
    }
}

pub(super) fn normalize(bits: i64, scalar: Scalar) -> i64 {
    match scalar {
        Scalar::Bool => i64::from(bits != 0),
        Scalar::Int16 => i64::from(bits as i16),
        Scalar::Uint16 => i64::from(bits as u16),
        Scalar::Int => i64::from(bits as i32),
        Scalar::Uint => i64::from(bits as u32),
        _ => bits,
    }
}

/// Read only literal spellings produced by integer::literal_text, plus source
/// literal bases/suffixes. Constructors and split minima here are literal IR
/// encodings, not a general expression parser.
pub(super) fn literal(text: &str) -> Option<i64> {
    match text {
        "(-2147483647 - 1)" => return Some(i64::from(i32::MIN)),
        "(-9223372036854775807ll - 1ll)" => return Some(i64::MIN),
        _ => {}
    }
    for prefix in ["int16_t(", "uint16_t("] {
        if let Some(inner) = text.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) {
            return literal(inner);
        }
    }
    if let Some(inner) = text.strip_prefix('-') {
        return literal(inner).map(i64::wrapping_neg);
    }
    if let Some(inner) = text.strip_prefix('+') {
        return literal(inner);
    }
    slang_lexer::parse_integer_literal(text)
        .ok()
        .map(|value| value.bits as i64)
}

pub(super) fn unary(op: &str, bits: i64, ty: Scalar) -> Option<i64> {
    Some(normalize(
        match op {
            "+" => bits,
            "-" => bits.wrapping_neg(),
            "~" => !bits,
            "!" => i64::from(bits == 0),
            _ => return None,
        },
        ty,
    ))
}

/// SCCP's evalLsh/evalRsh in the pinned source saturate at operand width,
/// including negative counts interpreted as unsigned. Typed arithmetic wraps
/// at its result width; division faults remain unknown rather than panicking.
pub(super) fn binary(op: &str, a: i64, b: i64, operand: Scalar, result: Scalar) -> Option<i64> {
    let unsigned = operand.is_unsigned();
    let bits = i64::from(operand.bit_width());
    let value = match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" if unsigned => (a as u64).checked_div(b as u64)? as i64,
        "%" if unsigned => (a as u64).checked_rem(b as u64)? as i64,
        "/" => a.checked_div(b)?,
        "%" => a.checked_rem(b)?,
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<<" if !(0..bits).contains(&b) => 0,
        ">>" if !(0..bits).contains(&b) => -i64::from(!unsigned && a < 0),
        "<<" => a.wrapping_shl(b as u32),
        ">>" if unsigned => ((a as u64) >> b) as i64,
        ">>" => a >> b,
        "==" => i64::from(a == b),
        "!=" => i64::from(a != b),
        "<" if unsigned => i64::from((a as u64) < b as u64),
        "<=" if unsigned => i64::from((a as u64) <= b as u64),
        ">" if unsigned => i64::from((a as u64) > b as u64),
        ">=" if unsigned => i64::from((a as u64) >= b as u64),
        "<" => i64::from(a < b),
        "<=" => i64::from(a <= b),
        ">" => i64::from(a > b),
        ">=" => i64::from(a >= b),
        _ => return None,
    };
    Some(normalize(value, result))
}
