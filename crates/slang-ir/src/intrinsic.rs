use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// Normalize conversion intrinsics and use vector output overloads for matrix rows.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_math_".to_owned();
    while names.iter().any(|n| n.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        functions: vec![],
        helpers: BTreeMap::new(),
    };
    lowerer.module(module)?;
    module.functions.extend(lowerer.functions);
    Ok(())
}

struct Lowerer {
    prefix: String,
    functions: Vec<Function>,
    helpers: BTreeMap<(String, Scalar, u8, u8), String>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        let ExprKind::IntrinsicCall { name, arguments } = &expression.kind else {
            return Ok(());
        };
        if matches!(name.as_str(), "asint" | "asuint" | "asfloat")
            && let [argument] = arguments.as_slice()
            && argument.ty == expression.ty
        {
            *expression = argument.clone();
            return Ok(());
        }
        if !matches!(name.as_str(), "sincos" | "modf" | "frexp") {
            return Ok(());
        }
        let Some(Expr {
            ty: Type::Matrix(scalar, rows, columns),
            ..
        }) = arguments.first()
        else {
            return Ok(());
        };
        let key = (name.clone(), *scalar, *rows, *columns);
        let helper = if let Some(helper) = self.helpers.get(&key) {
            helper.clone()
        } else {
            let helper = format!("{}{name}_{}", self.prefix, self.functions.len());
            self.functions.push(matrix_helper(
                &helper,
                name,
                *scalar,
                *rows,
                *columns,
                expression.span,
            ));
            self.helpers.insert(key, helper.clone());
            helper
        };
        expression.kind = ExprKind::Call {
            name: helper,
            arguments: arguments.clone(),
        };
        Ok(())
    }
}

fn matrix_helper(
    name: &str,
    intrinsic: &str,
    scalar: Scalar,
    rows: u8,
    columns: u8,
    span: Span,
) -> Function {
    let matrix = Type::Matrix(scalar, rows, columns);
    let result_type = if intrinsic == "sincos" {
        Type::Void
    } else {
        matrix.clone()
    };
    let output_type = if intrinsic == "frexp" {
        Type::Matrix(Scalar::Int, rows, columns)
    } else {
        matrix.clone()
    };
    let mut parameters = vec![Variable {
        name: "_input".into(),
        ty: matrix.clone(),
        modifiers: vec![],
        semantic: None,
        binding: None,
        initializer: None,
        span,
    }];
    let outputs = if intrinsic == "sincos" { 2 } else { 1 };
    for i in 0..outputs {
        parameters.push(Variable {
            name: format!("_out_{i}"),
            ty: output_type.clone(),
            modifiers: vec!["out".into()],
            semantic: None,
            binding: None,
            initializer: None,
            span,
        });
    }
    let mut body = vec![];
    if result_type != Type::Void {
        body.push(Stmt {
            span,
            kind: StmtKind::Variable(Variable {
                name: "_result".into(),
                ty: matrix.clone(),
                modifiers: vec![],
                semantic: None,
                binding: None,
                initializer: None,
                span,
            }),
        });
    }
    for row in 0..rows {
        let mut arguments = vec![matrix_row("_input", &matrix, row, span)];
        for i in 0..outputs {
            arguments.push(matrix_row(&format!("_out_{i}"), &output_type, row, span));
        }
        let call = Expr {
            ty: if intrinsic == "sincos" {
                Type::Void
            } else {
                Type::Vector(scalar, columns)
            },
            span,
            kind: ExprKind::IntrinsicCall {
                name: intrinsic.into(),
                arguments,
            },
        };
        let expression = if intrinsic == "sincos" {
            call
        } else {
            Expr {
                ty: call.ty.clone(),
                span,
                kind: ExprKind::Binary {
                    op: "=".into(),
                    left: Box::new(matrix_row("_result", &matrix, row, span)),
                    right: Box::new(call),
                },
            }
        };
        body.push(Stmt {
            span,
            kind: StmtKind::Expression(expression),
        });
    }
    if result_type != Type::Void {
        body.push(Stmt {
            span,
            kind: StmtKind::Return(Some(Expr {
                ty: matrix,
                span,
                kind: ExprKind::Name("_result".into()),
            })),
        });
    }
    Function {
        name: name.into(),
        return_type: result_type,
        parameters,
        modifiers: vec![],
        semantic: None,
        body: Some(Stmt {
            span,
            kind: StmtKind::Block(body),
        }),
        span,
    }
}

fn matrix_row(name: &str, matrix: &Type, row: u8, span: Span) -> Expr {
    let Type::Matrix(scalar, _, columns) = matrix else {
        unreachable!()
    };
    Expr {
        ty: Type::Vector(*scalar, *columns),
        span,
        kind: ExprKind::Index {
            base: Box::new(Expr {
                ty: matrix.clone(),
                span,
                kind: ExprKind::Name(name.into()),
            }),
            index: Box::new(Expr {
                ty: Type::Scalar(Scalar::Uint),
                span,
                kind: ExprKind::Integer(format!("{row}u")),
            }),
        },
    }
}
