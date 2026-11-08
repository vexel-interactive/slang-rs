//! Tokenization is independent of parsing and retains whitespace for preprocessing.

mod character;
mod lexer;
mod numeric;
mod token;

pub use character::{CharacterLiteralError, parse_character_literal};
pub use lexer::{Lexed, lex};
pub use numeric::{
    FloatKind, FloatLiteral, FloatLiteralError, integer_literal_value, parse_float_literal,
};
pub use token::{Token, TokenKind};
