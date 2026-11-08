//! Numeric literal values, independent of source locations and target syntax.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatKind {
    Half,
    Float,
    Double,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatLiteral {
    pub kind: FloatKind,
    /// The value after conversion to the literal's declared precision.
    pub value: f64,
    pub out_of_range: bool,
    /// Hexadecimal literals truncate discarded bits; decimal literals round.
    pub precision_lost: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLiteralError {
    Number,
    Suffix,
}

/// Parse an unsigned floating token using the pinned Slang literal rules.
pub fn parse_float_literal(text: &str) -> Result<FloatLiteral, FloatLiteralError> {
    let hex = text.starts_with("0x") || text.starts_with("0X");
    let body = if hex { &text[2..] } else { text };
    let number_end = if hex {
        if let Some(marker) = body.find(['p', 'P']) {
            let mut end = marker + 1;
            if matches!(body.as_bytes().get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            while body.as_bytes().get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            end
        } else {
            body.find('#').unwrap_or(body.len())
        }
    } else {
        body.find(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | 'e' | 'E' | '+' | '-'))
            .unwrap_or(body.len())
    };
    let (number, tail) = body.split_at(number_end);
    let (infinity, suffix) = tail
        .strip_prefix("#INF")
        .map_or((false, tail), |suffix| (true, suffix));
    let kind = match suffix {
        "" | "f" | "F" => FloatKind::Float,
        "h" | "H" | "hf" | "HF" | "fh" | "FH" => FloatKind::Half,
        "l" | "L" | "lf" | "LF" | "fl" | "FL" => FloatKind::Double,
        _ => return Err(FloatLiteralError::Suffix),
    };
    if infinity {
        return Ok(FloatLiteral {
            kind,
            value: f64::INFINITY,
            out_of_range: false,
            precision_lost: false,
        });
    }
    let (mut value, mut out_of_range, mut precision_lost) = if hex {
        hexadecimal(number)?
    } else {
        // Parse decimal float directly to f32, avoiding decimal -> f64 -> f32
        // double rounding at the halfway points.
        let value = if kind == FloatKind::Float {
            number.parse::<f32>().map(f64::from)
        } else {
            number.parse::<f64>()
        }
        .map_err(|_| FloatLiteralError::Number)?;
        if value.is_sign_negative() {
            return Err(FloatLiteralError::Number);
        }
        let nonzero = number
            .split(['e', 'E'])
            .next()
            .is_some_and(|s| s.bytes().any(|c| matches!(c, b'1'..=b'9')));
        (
            value,
            !value.is_finite() || (value == 0.0 && nonzero),
            false,
        )
    };
    let old = value;
    match kind {
        FloatKind::Half => value = narrow(value, -14, 15, 11, !hex),
        FloatKind::Float if hex => value = narrow(value, -126, 127, 24, false),
        _ => {}
    }
    out_of_range |= old != 0.0 && value == 0.0;
    // The source's hex path reports narrowing overflow as lost precision;
    // its decimal path reports overflow as out of range.
    out_of_range |= !hex && !value.is_finite();
    precision_lost |= hex && value != old;
    Ok(FloatLiteral {
        kind,
        value,
        out_of_range,
        precision_lost: precision_lost && !out_of_range,
    })
}

fn narrow(value: f64, minimum: i32, maximum: i32, precision: u32, round: bool) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let limit = 2.0_f64.powi(maximum + 1)
        - if round {
            2.0_f64.powi(maximum - precision as i32)
        } else {
            0.0
        };
    if value >= limit {
        return f64::INFINITY;
    }
    // Extract the exponent from the IEEE bits, including double subnormals.
    let bits = value.to_bits();
    let stored = ((bits >> 52) & 0x7ff) as i32;
    let exponent = if stored == 0 {
        63 - (bits & ((1_u64 << 52) - 1)).leading_zeros() as i32 - 1074
    } else {
        stored - 1023
    };
    let quantum = 2.0_f64.powi(exponent.max(minimum) - (precision as i32 - 1));
    let scaled = value / quantum;
    (if round {
        scaled.round_ties_even()
    } else {
        scaled.trunc()
    }) * quantum
}

fn hexadecimal(number: &str) -> Result<(f64, bool, bool), FloatLiteralError> {
    let (digits, exponent) = number.split_once(['p', 'P']).unwrap_or((number, "0"));
    let negative = exponent.starts_with('-');
    let exponent_digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
    if exponent_digits.is_empty() || !exponent_digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err(FloatLiteralError::Number);
    }
    let mut exponent = exponent_digits.bytes().fold(0_i64, |value, digit| {
        (value * 10 + i64::from(digit - b'0')).min(i64::MAX / 100)
    });
    if negative {
        exponent = -exponent;
    }
    let mut significand = 0_u64;
    let mut dot = false;
    let mut digit_seen = false;
    let mut precision_lost = false;
    for c in digits.chars() {
        if c == '.' && !dot {
            dot = true;
            continue;
        }
        let digit = c.to_digit(16).ok_or(FloatLiteralError::Number)?;
        digit_seen = true;
        if significand & 0xf000_0000_0000_0000 == 0 {
            significand = (significand << 4) | u64::from(digit);
            if dot {
                exponent -= 4;
            }
        } else {
            if !dot {
                exponent += 4;
            }
            precision_lost |= digit != 0;
        }
    }
    if !digit_seen {
        return Err(FloatLiteralError::Number);
    }
    if significand == 0 {
        return Ok((0.0, false, false));
    }
    let leading = significand.leading_zeros();
    significand <<= leading;
    exponent -= i64::from(leading);
    precision_lost |= significand & 0x7ff != 0;
    significand >>= 11;
    exponent += 11;
    if exponent < -1074 {
        let lost = -1074 - exponent;
        if lost > 52 {
            significand = 0;
            precision_lost = true;
        } else {
            precision_lost |= significand & ((1_u64 << lost) - 1) != 0;
            significand >>= lost;
        }
        exponent = -1074;
    }
    let value = (significand as f64 * 2.0_f64.powi(-52))
        * 2.0_f64.powi((exponent + 52).clamp(-1074, 1024) as i32);
    let out_of_range = value == 0.0 || !value.is_finite();
    Ok((value, out_of_range, precision_lost && !out_of_range))
}

/// Decode an integer token's magnitude. Width/type suffixes are checked by
/// the caller; this also serves preprocessor expressions.
pub fn integer_literal_value(text: &str) -> Option<u64> {
    let digits = text.trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(digits) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        u64::from_str_radix(digits, 16).ok()
    } else if let Some(digits) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        u64::from_str_radix(digits, 2).ok()
    } else if digits.len() > 1 && digits.starts_with('0') {
        u64::from_str_radix(digits, 8).ok()
    } else {
        digits.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_values_round_to_the_suffix_type() {
        for (text, kind, expected) in [
            (
                "1.000000059604644775390625000001",
                FloatKind::Float,
                f64::from(f32::from_bits(0x3f800001)),
            ),
            ("1.00048828125hf", FloatKind::Half, 1.0),
            ("1.00146484375FH", FloatKind::Half, 1.001953125),
            (
                "1.0000000000000002lf",
                FloatKind::Double,
                f64::from_bits(0x3ff0000000000001),
            ),
            ("65519.0h", FloatKind::Half, 65504.0),
            ("5.960464477539063e-8h", FloatKind::Half, 2.0_f64.powi(-24)),
        ] {
            let literal = parse_float_literal(text).unwrap();
            assert_eq!(literal.kind, kind, "{text}");
            assert_eq!(literal.value, expected, "{text}");
            assert!(!literal.out_of_range && !literal.precision_lost, "{text}");
        }
    }

    #[test]
    fn hexadecimal_values_truncate_and_preserve_subnormals() {
        for (text, expected, lost) in [
            ("0x1.8p+1", 3.0, false),
            ("0x1.8", 1.5, false),
            ("0x1.000003p0", 1.0 + 2.0_f64.powi(-23), true),
            ("0x1.003p0h", 1.0, true),
            (
                "0x1.00000000000018p0L",
                f64::from_bits(0x3ff0000000000001),
                true,
            ),
            ("0x1p-149", 2.0_f64.powi(-149), false),
            ("0x1p-24h", 2.0_f64.powi(-24), false),
            ("0x1p-1074L", f64::from_bits(1), false),
            ("0x1.fffffffffffffp1023L", f64::MAX, false),
            ("0x0p999999999999999999999999999999999", 0.0, false),
        ] {
            let literal = parse_float_literal(text).unwrap();
            assert_eq!(literal.value, expected, "{text}");
            assert_eq!(literal.precision_lost, lost, "{text}");
            assert!(!literal.out_of_range, "{text}");
        }
    }

    #[test]
    fn floating_range_and_spelling_errors_are_distinct() {
        for text in [
            "1e50f",
            "65520.0h",
            "1e9999L",
            "1e-9999",
            "0x1p-150",
            "0x1p-1075L",
        ] {
            assert!(parse_float_literal(text).unwrap().out_of_range, "{text}");
        }
        for text in ["1#INF", "0#INFf", "1.0#INFh", "1#INFL", "0x1#INF"] {
            let literal = parse_float_literal(text).unwrap();
            assert_eq!(literal.value, f64::INFINITY);
            assert!(!literal.out_of_range && !literal.precision_lost);
        }
        for text in ["1.0hF", "1.0Fl", "1.0u"] {
            assert_eq!(parse_float_literal(text), Err(FloatLiteralError::Suffix));
        }
        for text in ["1e+", "0x.p0", "0x1p-", "0x1.2.3p0"] {
            assert_eq!(parse_float_literal(text), Err(FloatLiteralError::Number));
        }
    }

    #[test]
    fn integer_bases_include_octal() {
        for (text, value) in [("010u", 8), ("0X10", 16), ("0b10", 2), ("10", 10), ("0", 0)] {
            assert_eq!(integer_literal_value(text), Some(value));
        }
        assert_eq!(integer_literal_value("08"), None);
    }
}
