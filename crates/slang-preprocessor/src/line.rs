use slang_lexer::{Token, TokenKind};
use slang_source::{Diagnostic, LineDirective, Span};

pub(crate) fn parse(tokens: &[Token]) -> Result<(LineDirective, usize), Diagnostic> {
    let Some(first) = tokens.first() else {
        return Ok((LineDirective::Default, 0));
    };
    if first.is("default") {
        return Ok((LineDirective::Default, 1));
    }
    if first.kind != TokenKind::Integer {
        return Err(error(
            first.span,
            "#line requires an integer literal or default",
        ));
    }
    let (name, consumed) = match tokens.get(1) {
        None => (None, 1),
        Some(token) if token.kind == TokenKind::String => (
            Some(decode_name(&token.text).map_err(|message| error(token.span, message))?),
            2,
        ),
        Some(token) if token.kind == TokenKind::Integer => (Some(token.text.clone()), 2),
        Some(token) => {
            return Err(error(
                token.span,
                "#line filename requires a string or integer literal",
            ));
        }
    };
    Ok((
        LineDirective::Remap {
            line: number(&first.text),
            name,
        },
        consumed,
    ))
}

fn error(span: Span, message: &'static str) -> Diagnostic {
    Diagnostic::error("P002", span, message)
}

/// Match upstream's signed strtoll conversion, including lower-case hex and suffixes.
pub(crate) fn number(text: &str) -> i32 {
    let negative = text.starts_with('-');
    let text = text.strip_prefix('-').unwrap_or(text);
    let (text, radix) = text
        .strip_prefix("0x")
        .map_or((text, 10), |text| (text, 16));
    let mut value = 0i64;
    for digit in text.chars().map_while(|c| c.to_digit(radix)) {
        value = value.saturating_mul(i64::from(radix));
        value = if negative {
            value.saturating_sub(i64::from(digit))
        } else {
            value.saturating_add(i64::from(digit))
        };
    }
    value as i32
}

fn decode_name(text: &str) -> Result<String, &'static str> {
    let text = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or("unterminated #line filename")?;
    let mut bytes = vec![];
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        let escaped = chars.next().ok_or("incomplete escape in #line filename")?;
        let byte = match escaped {
            '\'' => b'\'',
            '"' => b'"',
            '\\' => b'\\',
            '?' => b'?',
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
                u8::try_from(value).map_err(|_| "byte escape exceeds 255 in #line filename")?
            }
            'x' | 'u' | 'U' => {
                let braced = escaped != 'U' && chars.peek() == Some(&'{');
                if braced {
                    chars.next();
                }
                let limit = if braced || escaped == 'x' {
                    usize::MAX
                } else if escaped == 'u' {
                    4
                } else {
                    8
                };
                let mut value = 0u32;
                let mut digits = 0;
                while digits < limit {
                    let Some(digit) = chars.peek().and_then(|c| c.to_digit(16)) else {
                        break;
                    };
                    chars.next();
                    value = value
                        .checked_mul(16)
                        .and_then(|v| v.checked_add(digit))
                        .ok_or("escape value is too large in #line filename")?;
                    digits += 1;
                }
                if digits == 0
                    || braced && chars.next() != Some('}')
                    || !braced && escaped != 'x' && digits != limit
                {
                    return Err("invalid hexadecimal escape in #line filename");
                }
                if escaped != 'x' {
                    let c =
                        char::from_u32(value).ok_or("invalid Unicode scalar in #line filename")?;
                    let mut buffer = [0; 4];
                    bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                    continue;
                }
                u8::try_from(value).map_err(|_| "byte escape exceeds 255 in #line filename")?
            }
            _ => return Err("invalid escape in #line filename"),
        };
        bytes.push(byte);
    }
    String::from_utf8(bytes).map_err(|_| "#line filename escapes must form UTF-8")
}

/// Quote logical names without introducing raw line breaks or ambiguous byte escapes.
pub(crate) fn quote_name(name: &str) -> String {
    let mut result = "\"".to_owned();
    for c in name.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_ascii_control() => result.push_str(&format!("\\{:03o}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
