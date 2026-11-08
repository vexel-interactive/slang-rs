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
        let octal = !hex
            && !binary
            && self.text.as_bytes()[start] == b'0'
            && self
                .text
                .as_bytes()
                .get(start + 1)
                .is_some_and(u8::is_ascii_digit);
        if octal {
            self.diagnostics.push(Diagnostic::warning(
                "L005",
                Span::new(self.source, start, start + 1),
                "'0' prefix indicates an octal literal",
            ));
        }
        if hex || binary {
            self.offset += 2;
        }
        let digit_start = self.offset;
        self.number_digits(start, hex, binary, octal);
        if (hex || binary) && self.offset == digit_start {
            self.diagnostics.push(Diagnostic::error(
                "L004",
                Span::new(self.source, start, self.offset),
                "expected digits after numeric base prefix",
            ));
        }
        if self.current() == '.'
            && !matches!(self.text.as_bytes().get(self.offset + 1), Some(b'x' | b'r'))
        {
            float = true;
            self.bump();
            self.number_digits(start, hex, binary, octal);
        }
        if self.text[self.offset..].starts_with("#INF") {
            float = true;
            self.offset += 4;
        } else if (hex && matches!(self.current(), 'p' | 'P'))
            || (!hex && !binary && !octal && matches!(self.current(), 'e' | 'E'))
        {
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
        let suffix_start = self.offset;
        while self.offset < self.text.len()
            && (self.current().is_ascii_alphanumeric() || self.current() == '_')
        {
            self.bump();
        }
        let suffix = &self.text[suffix_start..self.offset];
        let valid_suffix = if float {
            matches!(
                suffix,
                "" | "f"
                    | "F"
                    | "h"
                    | "H"
                    | "hf"
                    | "HF"
                    | "fh"
                    | "FH"
                    | "l"
                    | "L"
                    | "lf"
                    | "LF"
                    | "fl"
                    | "FL"
            )
        } else {
            matches!(
                suffix.to_ascii_lowercase().as_str(),
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
        if float {
            TokenKind::Float
        } else {
            TokenKind::Integer
        }
    }

    fn number_digits(&mut self, start: usize, hex: bool, binary: bool, octal: bool) {
        while self.offset < self.text.len()
            && if hex {
                self.current().is_ascii_hexdigit()
            } else {
                self.current().is_ascii_digit()
            }
        {
            if (binary && !matches!(self.current(), '0' | '1'))
                || (octal && matches!(self.current(), '8' | '9'))
            {
                self.diagnostics.push(Diagnostic::error(
                    "L004",
                    Span::new(self.source, start, self.offset + 1),
                    "invalid digit for numeric base",
                ));
            }
            self.bump();
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

    #[test]
    fn floating_grammar_and_integer_members_keep_token_boundaries() {
        let text = "0e10 0E-3 .5 0x1.8p+2HF 0x1.8 1.0lf 1.0FL 0#INFh 123.x 0.r 0xff.x";
        let result = lex(SourceId(0), text);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let tokens: Vec<_> = result
            .tokens
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Whitespace
                        | TokenKind::Newline
                        | TokenKind::Comment
                        | TokenKind::Eof
                )
            })
            .collect();
        assert!(tokens[..8].iter().all(|t| t.kind == TokenKind::Float));
        for group in tokens[8..].chunks(3) {
            assert_eq!(group[0].kind, TokenKind::Integer);
            assert!(group[1].is("."));
            assert_eq!(group[2].kind, TokenKind::Identifier);
        }
        for text in ["2f", "2h", "1.0hF", "1.0lF", "0x1p+", "0b12", "09"] {
            assert!(
                lex(SourceId(0), text)
                    .diagnostics
                    .iter()
                    .any(|d| d.severity == slang_source::Severity::Error),
                "{text}"
            );
        }
    }
}
