//! Tokenization is independent of parsing and retains whitespace for preprocessing.

mod lexer;
mod token;

pub use lexer::{Lexed, lex};
pub use token::{Token, TokenKind};
