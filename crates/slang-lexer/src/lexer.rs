use crate::{Token, TokenKind};
use slang_source::{Diagnostic, SourceId, Span};

#[derive(Clone, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lex arbitrary UTF-8 input without panicking or discarding malformed characters.
pub fn lex(source: SourceId, text: &str) -> Lexed {
    let mut lexer = Lexer {
        source,
        text,
        offset: 0,
        tokens: vec![],
        diagnostics: vec![],
    };
    while lexer.offset < text.len() {
        lexer.next();
    }
    lexer.push(TokenKind::Eof, text.len());
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'a> {
    source: SourceId,
    text: &'a str,
    offset: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl Lexer<'_> {
    fn current(&self) -> char {
        self.text[self.offset..].chars().next().unwrap_or('\0')
    }

    fn bump(&mut self) {
        self.offset += self.current().len_utf8();
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            text: self.text[start..self.offset].into(),
            span: Span::new(self.source, start, self.offset),
        });
    }

    fn next(&mut self) {
        let start = self.offset;
        let remaining = &self.text[start..];
        let ch = self.current();
        let kind = if ch == '\n' || ch == '\r' {
            self.bump();
            if ch == '\r' && self.current() == '\n' {
                self.bump();
            }
            TokenKind::Newline
        } else if ch.is_whitespace() || (start == 0 && ch == '\u{feff}') {
            self.bump();
            while self.offset < self.text.len()
                && matches!(self.current(), ' ' | '\t' | '\u{b}' | '\u{c}')
            {
                self.bump();
            }
            TokenKind::Whitespace
        } else if remaining.starts_with("//") {
            while self.offset < self.text.len() && !matches!(self.current(), '\r' | '\n') {
                self.bump();
            }
            TokenKind::Comment
        } else if remaining.starts_with("/*") {
            self.offset += 2;
            while self.offset < self.text.len() && !self.text[self.offset..].starts_with("*/") {
                self.bump();
            }
            if self.offset == self.text.len() {
                self.diagnostics.push(Diagnostic::error(
                    "L001",
                    Span::new(self.source, start, self.offset),
                    "unterminated block comment",
                ));
            } else {
                self.offset += 2;
            }
            TokenKind::Comment
        } else if ch == '_' || ch.is_ascii_alphabetic() {
            self.bump();
            while self.offset < self.text.len()
                && (self.current().is_ascii_alphanumeric() || self.current() == '_')
            {
                self.bump();
            }
            TokenKind::Identifier
        } else if ch.is_ascii_digit()
            || (ch == '.' && remaining.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
        {
            self.number(start)
        } else if ch == '"' || ch == '\'' {
            self.bump();
            let mut terminated = false;
            while self.offset < self.text.len() && !matches!(self.current(), '\r' | '\n') {
                let c = self.current();
                self.bump();
                if c == ch {
                    terminated = true;
                    break;
                }
                if c == '\\' && self.offset < self.text.len() {
                    self.bump();
                }
            }
            if !terminated {
                self.diagnostics.push(Diagnostic::error(
                    "L002",
                    Span::new(self.source, start, self.offset),
                    "unterminated literal",
                ));
            }
            if ch == '"' {
                TokenKind::String
            } else {
                TokenKind::Character
            }
        } else {
            const OPERATORS: &[&str] = &[
                ">>=", "<<=", "...", "##", "::", "->", "++", "--", "&&", "||", "==", "!=", "<=",
                ">=", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
            ];
            if let Some(op) = OPERATORS.iter().find(|op| remaining.starts_with(**op)) {
                self.offset += op.len();
            } else {
                self.bump();
                if !"{}[]();,:.?~!+-*/%<>=&|^#\\".contains(ch) {
                    self.diagnostics.push(Diagnostic::error(
                        "L003",
                        Span::new(self.source, start, self.offset),
                        format!("unexpected character {ch:?}"),
                    ));
                }
            }
            TokenKind::Punctuation
        };
        self.push(kind, start);
    }

    fn number(&mut self, start: usize) -> TokenKind {
        let mut float = false;
        let hex = self.text[start..].starts_with("0x") || self.text[start..].starts_with("0X");
        let binary = self.text[start..].starts_with("0b") || self.text[start..].starts_with("0B");
        if hex || binary {
            self.offset += 2;
        }
        let digit_start = self.offset;
        while self.offset < self.text.len()
            && if hex {
                self.current().is_ascii_hexdigit()
            } else if binary {
                matches!(self.current(), '0' | '1')
            } else {
                self.current().is_ascii_digit()
            }
        {
            self.bump();
        }
        if (hex || binary) && self.offset == digit_start {
            self.diagnostics.push(Diagnostic::error(
                "L004",
                Span::new(self.source, start, self.offset),
                "expected digits after numeric base prefix",
            ));
        }
        if !hex && !binary {
            if self.current() == '.' {
                float = true;
                self.bump();
                while self.offset < self.text.len() && self.current().is_ascii_digit() {
                    self.bump();
                }
            }
            if matches!(self.current(), 'e' | 'E') {
                float = true;
                self.bump();
                if matches!(self.current(), '+' | '-') {
                    self.bump();
                }
                let exponent = self.offset;
                while self.offset < self.text.len() && self.current().is_ascii_digit() {
                    self.bump();
                }
                if self.offset == exponent {
                    self.diagnostics.push(Diagnostic::error(
                        "L004",
                        Span::new(self.source, start, self.offset),
                        "expected exponent digits",
                    ));
                }
            }
        }
        let suffix_start = self.offset;
        if matches!(self.current(), 'f' | 'F' | 'h' | 'H') && !hex && !binary {
            float = true;
            self.bump();
        } else {
            while self.offset < self.text.len() && matches!(self.current(), 'u' | 'U' | 'l' | 'L') {
                self.bump();
            }
        }
        let suffix = self.text[suffix_start..self.offset].to_ascii_lowercase();
        let valid_suffix = if float {
            matches!(suffix.as_str(), "" | "f" | "h")
        } else {
            matches!(
                suffix.as_str(),
                "" | "u" | "l" | "ul" | "lu" | "ll" | "ull" | "llu"
            )
        };
        if !valid_suffix {
            self.diagnostics.push(Diagnostic::error(
                "L004",
                Span::new(self.source, start, self.offset),
                "invalid numeric suffix",
            ));
        }
        if self.current().is_ascii_alphanumeric() || self.current() == '_' {
            while self.offset < self.text.len()
                && (self.current().is_ascii_alphanumeric() || self.current() == '_')
            {
                self.bump();
            }
            self.diagnostics.push(Diagnostic::error(
                "L004",
                Span::new(self.source, start, self.offset),
                "invalid numeric literal",
            ));
        }
        if float {
            TokenKind::Float
        } else {
            TokenKind::Integer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_match_and_round_trip() {
        let text = "// test\r\nuint x = 0xffu; x >>= 2; float f=.5e+2f; /*α*/";
        let result = lex(SourceId(0), text);
        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|t| t.text.as_str())
                .collect::<String>(),
            text
        );
        assert!(result.tokens.iter().any(|t| t.is(">>=")));
    }

    #[test]
    fn malformed_input_is_diagnosed() {
        for text in ["1uu", "1.0u"] {
            assert!(!lex(SourceId(0), text).diagnostics.is_empty());
        }
        for text in ["/*", "\"unterminated", "α", "1e+", "0x", "0b2", "12wat"] {
            assert!(!lex(SourceId(0), text).diagnostics.is_empty(), "{text}");
        }
    }
}
