//! Syntax trees retain spans and expression IDs for later type checking.

mod ast;
mod declaration;
mod expression;
mod parser;
mod statement;

pub use ast::*;
pub use parser::parse;
