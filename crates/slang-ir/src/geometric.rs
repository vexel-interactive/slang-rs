use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::Diagnostic;
use std::collections::BTreeMap;

/// Keep integer cross products in integer arithmetic and select HLSL float operations.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_geometric_".to_owned();
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
    helpers: BTreeMap<Scalar, String>,
    functions: Vec<Function>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        let ExprKind::IntrinsicCall { name, arguments } = &expression.kind else {
            return Ok(());
        };
        if name == "cross" && expression.ty.integral() {
            let scalar = expression.ty.scalar().unwrap();
            let helper = if let Some(helper) = self.helpers.get(&scalar) {
                helper.clone()
            } else {
                let helper = format!("{}cross_{}", self.prefix, self.functions.len());
                self.functions
                    .push(integer_cross(&helper, &expression.ty, expression.span));
                self.helpers.insert(scalar, helper.clone());
                helper
            };
            expression.kind = ExprKind::Call {
                name: helper,
                arguments: arguments.clone(),
            };
        } else if matches!(
            name.as_str(),
            "cross" | "distance" | "reflect" | "refract" | "faceforward" | "length" | "normalize"
        ) && expression.ty.scalar() == Some(Scalar::Double)
        {
            let arguments = arguments
                .iter()
                .map(|argument| Expr {
                    ty: argument.ty.with_scalar(Scalar::Float),
                    span: argument.span,
                    kind: ExprKind::Cast {
                        value: Box::new(argument.clone()),
                    },
                })
                .collect();
            expression.kind = ExprKind::Cast {
                value: Box::new(Expr {
                    ty: expression.ty.with_scalar(Scalar::Float),
                    span: expression.span,
                    kind: ExprKind::IntrinsicCall {
                        name: name.clone(),
                        arguments,
                    },
                }),
            };
        }
        Ok(())
    }
}

fn integer_cross(name: &str, ty: &Type, span: slang_source::Span) -> Function {
    let component_product = |left: &str, right: &str| Expr {
        ty: ty.clone(),
        span,
        kind: ExprKind::Binary {
            op: "*".into(),
            left: Box::new(Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Member {
                    base: Box::new(Expr {
                        ty: ty.clone(),
                        span,
                        kind: ExprKind::Name("_left".into()),
                    }),
                    name: left.into(),
                },
            }),
            right: Box::new(Expr {
                ty: ty.clone(),
                span,
                kind: ExprKind::Member {
                    base: Box::new(Expr {
                        ty: ty.clone(),
                        span,
                        kind: ExprKind::Name("_right".into()),
                    }),
                    name: right.into(),
                },
            }),
        },
    };
    let result = Expr {
        ty: ty.clone(),
        span,
        kind: ExprKind::Binary {
            op: "-".into(),
            left: Box::new(component_product("yzx", "zxy")),
            right: Box::new(component_product("zxy", "yzx")),
        },
    };
    Function {
        name: name.into(),
        return_type: ty.clone(),
        span,
        parameters: ["_left", "_right"]
            .into_iter()
            .map(|name| Variable {
                name: name.into(),
                ty: ty.clone(),
                modifiers: vec![],
                semantic: None,
                initializer: None,
                span,
            })
            .collect(),
        modifiers: vec![],
        semantic: None,
        body: Some(Stmt {
            span,
            kind: StmtKind::Block(vec![Stmt {
                span,
                kind: StmtKind::Return(Some(result)),
            }]),
        }),
    }
}
