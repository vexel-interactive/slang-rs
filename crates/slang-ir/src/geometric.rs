use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// Preserve arithmetic reductions and select HLSL float geometric operations.
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
    helpers: BTreeMap<(String, String), String>,
    functions: Vec<Function>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        let ExprKind::IntrinsicCall { name, arguments } = &expression.kind else {
            return Ok(());
        };
        if name == "cross" && expression.ty.integral()
            || name == "dot"
                && matches!(
                    expression.ty.scalar(),
                    Some(
                        Scalar::Int16
                            | Scalar::Uint16
                            | Scalar::Int
                            | Scalar::Uint
                            | Scalar::Int64
                            | Scalar::Uint64
                            | Scalar::IntPtr
                            | Scalar::UintPtr
                            | Scalar::Double
                    )
                )
        {
            let parameter = &arguments[0].ty;
            let key = (name.clone(), parameter.to_string());
            let helper = if let Some(helper) = self.helpers.get(&key) {
                helper.clone()
            } else {
                let helper = format!("{}{name}_{}", self.prefix, self.functions.len());
                let function = if name == "cross" {
                    integer_cross(&helper, parameter, expression.span)
                } else {
                    dot(&helper, parameter, expression.span)
                };
                self.functions.push(function);
                self.helpers.insert(key, helper.clone());
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

fn integer_cross(name: &str, ty: &Type, span: Span) -> Function {
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
                binding: None,
                uniform_layout: None,
                pack_offset: None,
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

/// A scalar reduction keeps the selected integer or double arithmetic in every component.
fn dot(name: &str, parameter: &Type, span: Span) -> Function {
    let result_type = Type::Scalar(parameter.scalar().unwrap());
    let component = |binding: &str, index: usize| {
        let value = Expr {
            ty: parameter.clone(),
            span,
            kind: ExprKind::Name(binding.into()),
        };
        if matches!(parameter, Type::Vector(..)) {
            Expr {
                ty: result_type.clone(),
                span,
                kind: ExprKind::Member {
                    base: Box::new(value),
                    name: ["x", "y", "z", "w"][index].into(),
                },
            }
        } else {
            value
        }
    };
    let mut products = (0..usize::from(parameter.width().unwrap())).map(|index| Expr {
        ty: result_type.clone(),
        span,
        kind: ExprKind::Binary {
            op: "*".into(),
            left: Box::new(component("_left", index)),
            right: Box::new(component("_right", index)),
        },
    });
    let mut result = products.next().unwrap();
    for product in products {
        result = Expr {
            ty: result_type.clone(),
            span,
            kind: ExprKind::Binary {
                op: "+".into(),
                left: Box::new(result),
                right: Box::new(product),
            },
        };
    }
    Function {
        name: name.into(),
        return_type: result_type,
        span,
        parameters: ["_left", "_right"]
            .into_iter()
            .map(|name| Variable {
                name: name.into(),
                ty: parameter.clone(),
                modifiers: vec![],
                semantic: None,
                binding: None,
                uniform_layout: None,
                pack_offset: None,
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
