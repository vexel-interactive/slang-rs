//! Tokenization is independent of parsing and retains whitespace for preprocessing.

mod lexer;
mod numeric;
mod token;

pub use lexer::{Lexed, lex};
pub use numeric::{
    FloatKind, FloatLiteral, FloatLiteralError, integer_literal_value, parse_float_literal,
};
pub use token::{Token, TokenKind};
