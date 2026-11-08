//! Tokenization is independent of parsing and retains whitespace for preprocessing.

mod character;
mod integer;
mod lexer;
mod numeric;
pub use integer::{
    IntegerKind, IntegerLiteral, IntegerLiteralError, IntegerSuffix, IntegerWidth,
    parse_integer_literal, parse_integer_suffix,
};
mod token;

pub use character::{CharacterLiteralError, parse_character_literal};
pub use lexer::{Lexed, lex};
pub use numeric::{
    FloatKind, FloatLiteral, FloatLiteralError, integer_literal_value, parse_float_literal,
};
pub use token::{Token, TokenKind};
