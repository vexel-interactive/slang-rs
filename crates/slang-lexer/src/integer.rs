//! Integer suffix parsing and type selection from the pinned Slang parser.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerWidth {
    Default,
    Long,
    LongLong,
    Pointer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerSuffix {
    pub width: IntegerWidth,
    pub unsigned: bool,
}

/// A width and one unsigned marker may appear in either order. The two
/// letters of a long-long suffix must have matching case.
pub fn parse_integer_suffix(suffix: &str) -> Option<IntegerSuffix> {
    let mut result = IntegerSuffix {
        width: IntegerWidth::Default,
        unsigned: false,
    };
    let mut chars = suffix.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'u' | 'U' if !result.unsigned => result.unsigned = true,
            'l' | 'L' if result.width == IntegerWidth::Default => {
                result.width = if chars.next_if_eq(&c).is_some() {
                    IntegerWidth::LongLong
                } else {
                    IntegerWidth::Long
                };
            }
            'z' | 'Z' if result.width == IntegerWidth::Default => {
                result.width = IntegerWidth::Pointer;
            }
            _ => return None,
        }
    }
    Some(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerKind {
    Int,
    Uint,
    Int64,
    Uint64,
    IntPtr,
    UintPtr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerLiteralError {
    Number,
    Suffix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerLiteral {
    pub kind: IntegerKind,
    /// Bit pattern, including values outside the signed 64-bit range.
    pub bits: u64,
    pub signed_minimum_exception: bool,
    pub signed_overflow: bool,
}

impl IntegerLiteral {
    /// Fold a parser prefix, preserving the literal type and minimum exception.
    pub fn prefix(mut self, op: &str) -> Self {
        if op == "-" && self.signed_minimum_exception {
            self.kind = if self.kind == IntegerKind::UintPtr {
                IntegerKind::IntPtr
            } else if self.bits == 1_u64 << 31 {
                IntegerKind::Int
            } else {
                IntegerKind::Int64
            };
            self.signed_overflow = false;
        }
        self.bits = match op {
            "-" => self.bits.wrapping_neg(),
            "~" => !self.bits,
            _ => self.bits,
        };
        self.signed_minimum_exception = false;
        self
    }
}

pub fn parse_integer_literal(text: &str) -> Result<IntegerLiteral, IntegerLiteralError> {
    let digits = text.trim_end_matches(['u', 'U', 'l', 'L', 'z', 'Z']);
    let suffix = parse_integer_suffix(&text[digits.len()..]).ok_or(IntegerLiteralError::Suffix)?;
    let bits = crate::integer_literal_value(text).ok_or(IntegerLiteralError::Number)?;
    let decimal = !digits.starts_with(['0']) || digits == "0";
    let pointer = suffix.width == IntegerWidth::Pointer;
    let narrow = matches!(suffix.width, IntegerWidth::Default | IntegerWidth::Long);
    let kind = if pointer {
        if !suffix.unsigned && (!decimal || bits <= i64::MAX as u64) {
            IntegerKind::IntPtr
        } else {
            IntegerKind::UintPtr
        }
    } else if narrow && !suffix.unsigned && bits <= i32::MAX as u64 {
        IntegerKind::Int
    } else if narrow && (suffix.unsigned || !decimal) && bits <= u32::MAX as u64 {
        IntegerKind::Uint
    } else if !suffix.unsigned && bits <= i64::MAX as u64 {
        IntegerKind::Int64
    } else {
        IntegerKind::Uint64
    };
    let signed_overflow = decimal && !suffix.unsigned && bits > i64::MAX as u64;
    Ok(IntegerLiteral {
        kind,
        bits,
        signed_minimum_exception: decimal
            && !suffix.unsigned
            && ((narrow && bits == 1_u64 << 31) || bits == 1_u64 << 63),
        signed_overflow,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_widths_preserve_case_and_reject_repeated_markers() {
        for text in ["ll", "LL", "uLL", "Ull", "LLu", "llU"] {
            assert_eq!(
                parse_integer_suffix(text).unwrap().width,
                IntegerWidth::LongLong,
                "{text}"
            );
        }
        for text in ["lL", "Ll", "uu", "uU", "lul", "zL", "LLz", "f"] {
            assert!(parse_integer_suffix(text).is_none(), "{text}");
        }
        assert_eq!(
            parse_integer_suffix("Zu"),
            Some(IntegerSuffix {
                width: IntegerWidth::Pointer,
                unsigned: true
            })
        );
    }

    #[test]
    fn decimal_and_nondecimal_boundaries_select_distinct_types() {
        for (text, kind) in [
            ("2147483647", IntegerKind::Int),
            ("2147483648", IntegerKind::Int64),
            ("4294967295u", IntegerKind::Uint),
            ("4294967296u", IntegerKind::Uint64),
            ("9223372036854775807L", IntegerKind::Int64),
            ("9223372036854775808", IntegerKind::Uint64),
            ("18446744073709551615ULL", IntegerKind::Uint64),
            ("0x80000000", IntegerKind::Uint),
            ("0x100000000", IntegerKind::Int64),
            ("0x8000000000000000LL", IntegerKind::Uint64),
            ("020000000000", IntegerKind::Uint),
            ("0b10000000000000000000000000000000", IntegerKind::Uint),
            ("1ll", IntegerKind::Int64),
        ] {
            assert_eq!(parse_integer_literal(text).unwrap().kind, kind, "{text}");
        }
        assert!(parse_integer_literal("18446744073709551616").is_err());
    }

    #[test]
    fn prefix_minus_consumes_only_the_signed_minimum_exception() {
        let minimum = parse_integer_literal("2147483648").unwrap().prefix("-");
        assert_eq!(minimum.kind, IntegerKind::Int);
        assert_eq!(minimum.bits as i64, -2147483648);
        assert_eq!(
            parse_integer_literal("2147483648LL")
                .unwrap()
                .prefix("-")
                .kind,
            IntegerKind::Int64
        );
        assert_eq!(
            parse_integer_literal("2147483648")
                .unwrap()
                .prefix("+")
                .prefix("-")
                .kind,
            IntegerKind::Int64
        );
        let minimum = parse_integer_literal("9223372036854775808")
            .unwrap()
            .prefix("-");
        assert_eq!(minimum.kind, IntegerKind::Int64);
        assert_eq!(minimum.bits as i64, i64::MIN);
        assert!(!minimum.signed_overflow);
        assert!(
            parse_integer_literal("9223372036854775808")
                .unwrap()
                .prefix("~")
                .signed_overflow
        );
        assert!(
            parse_integer_literal("9223372036854775809")
                .unwrap()
                .prefix("-")
                .signed_overflow
        );
    }

    #[test]
    fn pointer_literals_preserve_identity_and_nondecimal_signed_bits() {
        for (text, kind) in [
            ("1Z", IntegerKind::IntPtr),
            ("2147483648z", IntegerKind::IntPtr),
            ("9223372036854775808Z", IntegerKind::UintPtr),
            ("0xffffffffffffffffZ", IntegerKind::IntPtr),
            ("01777777777777777777777z", IntegerKind::IntPtr),
            ("18446744073709551615UZ", IntegerKind::UintPtr),
            ("1zU", IntegerKind::UintPtr),
        ] {
            assert_eq!(parse_integer_literal(text).unwrap().kind, kind, "{text}");
        }
        let minimum = parse_integer_literal("9223372036854775808Z")
            .unwrap()
            .prefix("-");
        assert_eq!(minimum.kind, IntegerKind::IntPtr);
        assert_eq!(minimum.bits as i64, i64::MIN);
        assert!(!minimum.signed_overflow);
        let smaller = parse_integer_literal("2147483648Z").unwrap().prefix("-");
        assert_eq!(smaller.kind, IntegerKind::IntPtr);
        assert_eq!(smaller.bits as i64, -2147483648);
        assert!(parse_integer_literal("18446744073709551616UZ").is_err());
    }
}
