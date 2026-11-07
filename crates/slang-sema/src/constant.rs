use crate::{Scalar, Type, checker::Checker};
use slang_syntax::{Expr, ExprKind};

impl Checker {
    /// Fold the implemented scalar integer constant subset using 32-bit shapes.
    pub(crate) fn integer_constant(&self, expr: &Expr) -> Option<i64> {
        let value = match &expr.kind {
            ExprKind::Integer(text) => integer(text)?,
            ExprKind::Bool(value) => i64::from(*value),
            ExprKind::Name(name) => self.lookup(name)?.constant?,
            ExprKind::Unary { op, value } => {
                let value = self.integer_constant(value)?;
                match op.as_str() {
                    "+" => value,
                    "-" => value.wrapping_neg(),
                    "!" => i64::from(value == 0),
                    "~" => !value,
                    _ => return None,
                }
            }
            ExprKind::Binary { op, left, right } => {
                let mut a = self.integer_constant(left)?;
                if op == "&&" && a == 0 {
                    return Some(0);
                }
                if op == "||" && a != 0 {
                    return Some(1);
                }
                let mut b = self.integer_constant(right)?;
                if !matches!(op.as_str(), "<<" | ">>")
                    && let Some(common) = self
                        .expression_types
                        .get(&left.id)
                        .zip(self.expression_types.get(&right.id))
                        .and_then(|(a, b)| Type::common(a, b))
                {
                    a = normalize(a, &common);
                    b = normalize(b, &common);
                }
                match op.as_str() {
                    "+" => a.wrapping_add(b),
                    "-" => a.wrapping_sub(b),
                    "*" => a.wrapping_mul(b),
                    "/" => a.checked_div(b)?,
                    "%" => a.checked_rem(b)?,
                    "&" => a & b,
                    "|" => a | b,
                    "^" => a ^ b,
                    "<<" if (0..32).contains(&b) => a << b,
                    ">>" if (0..32).contains(&b) => a >> b,
                    "==" => i64::from(a == b),
                    "!=" => i64::from(a != b),
                    "<" => i64::from(a < b),
                    "<=" => i64::from(a <= b),
                    ">" => i64::from(a > b),
                    ">=" => i64::from(a >= b),
                    "&&" => i64::from(a != 0 && b != 0),
                    "||" => i64::from(a != 0 || b != 0),
                    _ => return None,
                }
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => self.integer_constant(if self.integer_constant(condition)? != 0 {
                then_value
            } else {
                else_value
            })?,
            ExprKind::Cast { value, .. } => self.constant_conversion(value)?,
            ExprKind::Call { callee, arguments }
                if arguments.len() == 1
                    && matches!(&callee.kind, ExprKind::Name(name) if matches!(name.as_str(), "int" | "uint" | "bool")) =>
            {
                self.constant_conversion(&arguments[0])?
            }
            _ => return None,
        };
        Some(
            self.expression_types
                .get(&expr.id)
                .map_or(value, |ty| normalize(value, ty)),
        )
    }

    fn constant_conversion(&self, expr: &Expr) -> Option<i64> {
        if let ExprKind::Float(text) = &expr.kind {
            let value = text
                .trim_end_matches(['f', 'F', 'h', 'H'])
                .parse::<f64>()
                .ok()?;
            (value.is_finite() && value >= i32::MIN as f64 && value <= u32::MAX as f64)
                .then_some(value as i64)
        } else {
            self.integer_constant(expr)
        }
    }
}

pub(crate) fn normalize(value: i64, ty: &Type) -> i64 {
    match ty {
        Type::Scalar(Scalar::Int) => i64::from(value as i32),
        Type::Scalar(Scalar::Uint) => i64::from(value as u32),
        Type::Scalar(Scalar::Bool) => i64::from(value != 0),
        _ => value,
    }
}

fn integer(text: &str) -> Option<i64> {
    let text = text.trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(text) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(text, 16).ok()
    } else if let Some(text) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        i64::from_str_radix(text, 2).ok()
    } else {
        text.parse().ok()
    }
}
