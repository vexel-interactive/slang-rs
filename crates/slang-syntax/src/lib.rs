//! Syntax trees retain spans and expression IDs for later type checking.

mod ast;
mod binding;
mod constant_buffer;
mod declaration;
mod expression;
mod module;
mod parser;
mod statement;
mod visibility;

pub use ast::*;
pub use parser::parse;
