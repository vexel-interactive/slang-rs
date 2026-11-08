//! Slang character literals carry one unsigned 32-bit value.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterLiteralError {
    CharacterCount,
    Escape,
    Overflow,
}

/// Decode one character or one escape. Unlike a Unicode string escape, a
/// character escape may name any uint32 value, including surrogate values.
pub fn parse_character_literal(text: &str) -> Result<u32, CharacterLiteralError> {
    let body = text
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .ok_or(CharacterLiteralError::CharacterCount)?;
    let mut chars = body.chars().peekable();
    let first = chars.next().ok_or(CharacterLiteralError::CharacterCount)?;
    let value = if first == '\\' {
        let escaped = chars.next().ok_or(CharacterLiteralError::Escape)?;
        match escaped {
            '\'' => 39,
            '"' => 34,
            '\\' => 92,
            '?' => 63,
            'a' => 7,
            'b' => 8,
            'f' => 12,
            'n' => 10,
            'r' => 13,
            't' => 9,
            'v' => 11,
            '0'..='7' => {
                let mut value = escaped.to_digit(8).unwrap();
                for _ in 0..2 {
                    let Some(digit) = chars.peek().and_then(|c| c.to_digit(8)) else {
                        break;
                    };
                    chars.next();
                    value = value * 8 + digit;
                }
                value
            }
            'x' | 'u' | 'U' => {
                let braced = escaped != 'U' && chars.peek() == Some(&'{');
                if braced {
                    chars.next();
                }
                let limit = if braced || escaped == 'x' {
                    255
                } else if escaped == 'u' {
                    4
                } else {
                    8
                };
                let mut value = 0_u32;
                let mut digits = 0;
                while digits < limit {
                    let Some(digit) = chars.peek().and_then(|c| c.to_digit(16)) else {
                        break;
                    };
                    chars.next();
                    value = value
                        .checked_mul(16)
                        .and_then(|v| v.checked_add(digit))
                        .ok_or(CharacterLiteralError::Overflow)?;
                    digits += 1;
                }
                if digits == 0
                    || braced && chars.next() != Some('}')
                    || !braced && escaped != 'x' && digits != limit
                {
                    return Err(CharacterLiteralError::Escape);
                }
                value
            }
            _ => return Err(CharacterLiteralError::Escape),
        }
    } else {
        if matches!(first, '\'' | '\n' | '\r') {
            return Err(CharacterLiteralError::CharacterCount);
        }
        u32::from(first)
    };
    if chars.next().is_some() {
        return Err(CharacterLiteralError::CharacterCount);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_values_follow_the_pinned_literal_corpus() {
        for (literal, value) in [
            ("'X'", 0x58),
            ("'\u{e4}'", 0xe4),
            ("'\u{20ac}'", 0x20ac),
            ("'\u{10fffd}'", 0x10fffd),
            (r"'\n'", 10),
            (r"'\0'", 0),
            (r"'\377'", 255),
            (r"'\777'", 511),
            (r"'\xFEDC1234'", 0xfedc1234),
            (r"'\x{ffffffff}'", u32::MAX),
            (r"'\u1234'", 0x1234),
            (r"'\u{00fefefefe}'", 0xfefefefe),
            (r"'\Uffffffff'", u32::MAX),
            (r"'\u{d800}'", 0xd800),
            (r"'\''", 39),
            (r"'\\'", 92),
        ] {
            assert_eq!(parse_character_literal(literal), Ok(value), "{literal}");
        }
    }

    #[test]
    fn malformed_character_escapes_and_multiple_values_fail() {
        for literal in [
            "''",
            "'ab'",
            r"'\q'",
            r"'\x'",
            r"'\u12'",
            r"'\U1234'",
            r"'\x{}'",
            r"'\u{12'",
            r"'\x{12z}'",
            r"'\U{12}'",
            r"'\1234'",
            r"'\x100000000'",
            r"'\u{100000000}'",
            r"'\u12345'",
            "'\n'",
        ] {
            assert!(parse_character_literal(literal).is_err(), "{literal}");
        }
        assert!(parse_character_literal(&format!("'\\x{}'", "0".repeat(256))).is_err());
    }
}
