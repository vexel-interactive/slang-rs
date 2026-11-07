use serde::Serialize;
use slang_source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TokenKind {
    Identifier,
    Integer,
    Float,
    String,
    Character,
    Punctuation,
    Whitespace,
    Newline,
    Comment,
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

impl Token {
    pub fn is(&self, text: &str) -> bool {
        self.text == text
    }

    pub fn is_trivia(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Whitespace | TokenKind::Newline | TokenKind::Comment
        )
    }
}
