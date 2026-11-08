use crate::{Expr, ExprKind};
use slang_lexer::{IntegerLiteral, IntegerLiteralError, parse_integer_literal};

/// Prefixes immediately around integer literals are folded by the upstream
/// parser. Parentheses around the operand preserve an ordinary unary expression.
pub fn integer_literal(expr: &Expr) -> Result<Option<IntegerLiteral>, IntegerLiteralError> {
    match &expr.kind {
        ExprKind::Integer(text) => parse_integer_literal(text).map(Some),
        ExprKind::Unary { op, value }
            if matches!(op.as_str(), "+" | "-" | "~") && !value.parenthesized =>
        {
            Ok(integer_literal(value)?.map(|literal| literal.prefix(op)))
        }
        _ => Ok(None),
    }
}
