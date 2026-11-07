use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// HLSL has no coarse/fine fwidth; helpers preserve one evaluation of its input.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_derivative_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        helpers: BTreeMap::new(),
        functions: vec![],
    };
    lowerer.module(module)?;
    module.functions.extend(lowerer.functions);
    Ok(())
}

struct Lowerer {
    prefix: String,
    helpers: BTreeMap<(String, String), String>,
    functions: Vec<Function>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        let ExprKind::IntrinsicCall { name, arguments } = &expression.kind else {
            return Ok(());
        };
        if matches!(name.as_str(), "fwidth_coarse" | "fwidth_fine") {
            let key = (name.clone(), expression.ty.to_string());
            let helper = if let Some(helper) = self.helpers.get(&key) {
                helper.clone()
            } else {
                let helper = format!("{}{name}_{}", self.prefix, self.functions.len());
                self.functions
                    .push(width_helper(&helper, name, &expression.ty, expression.span));
                self.helpers.insert(key, helper.clone());
                helper
            };
            expression.kind = ExprKind::Call {
                name: helper,
                arguments: arguments.clone(),
            };
        } else if matches!(
            name.as_str(),
            "ddx" | "ddy" | "ddx_coarse" | "ddy_coarse" | "ddx_fine" | "ddy_fine" | "fwidth"
        ) && expression.ty.scalar() == Some(Scalar::Double)
        {
            *expression = derivative_value(name, arguments[0].clone(), expression.span);
        }
        Ok(())
    }
}

fn derivative_value(name: &str, input: Expr, span: Span) -> Expr {
    let ty = input.ty.clone();
    let input = if ty.scalar() == Some(Scalar::Double) {
        Expr {
            ty: ty.with_scalar(Scalar::Float),
            span,
            kind: ExprKind::Cast {
                value: Box::new(input),
            },
        }
    } else {
        input
    };
    let call = Expr {
        ty: input.ty.clone(),
        span,
        kind: ExprKind::IntrinsicCall {
            name: name.into(),
            arguments: vec![input],
        },
    };
    if call.ty == ty {
        call
    } else {
        Expr {
            ty,
            span,
            kind: ExprKind::Cast {
                value: Box::new(call),
            },
        }
    }
}

fn width_helper(name: &str, intrinsic: &str, ty: &Type, span: Span) -> Function {
    let suffix = intrinsic.strip_prefix("fwidth").unwrap();
    let input = Expr {
        ty: ty.clone(),
        span,
        kind: ExprKind::Name("_input".into()),
    };
    let magnitudes: Vec<_> = ["ddx", "ddy"]
        .into_iter()
        .map(|axis| Expr {
            ty: ty.clone(),
            span,
            kind: ExprKind::IntrinsicCall {
                name: "abs".into(),
                arguments: vec![derivative_value(
                    &format!("{axis}{suffix}"),
                    input.clone(),
                    span,
                )],
            },
        })
        .collect();
    let mut magnitudes = magnitudes.into_iter();
    let result = Expr {
        ty: ty.clone(),
        span,
        kind: ExprKind::Binary {
            op: "+".into(),
            left: Box::new(magnitudes.next().unwrap()),
            right: Box::new(magnitudes.next().unwrap()),
        },
    };
    Function {
        name: name.into(),
        return_type: ty.clone(),
        parameters: vec![Variable {
            name: "_input".into(),
            ty: ty.clone(),
            modifiers: vec![],
            semantic: None,
            binding: None,
            initializer: None,
            span,
        }],
        modifiers: vec![],
        semantic: None,
        span,
        body: Some(Stmt {
            span,
            kind: StmtKind::Block(vec![Stmt {
                span,
                kind: StmtKind::Return(Some(result)),
            }]),
        }),
    }
}
