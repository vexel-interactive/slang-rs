mod declaration_default;

use crate::*;
use slang_sema::{Scalar, Type};
use slang_source::Diagnostic;
use std::collections::{BTreeMap, BTreeSet};

/// Lower aggregate construction to ordinary typed functions and field writes.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let mut names = BTreeSet::new();
    for structure in &module.structures {
        names.insert(structure.name.clone());
        names.extend(structure.fields.iter().map(|f| f.name.clone()));
    }
    for global in &module.globals {
        names.insert(global.name.clone());
    }
    for function in &module.functions {
        names.insert(function.name.clone());
        names.extend(function.parameters.iter().map(|p| p.name.clone()));
        if let Some(body) = &function.body {
            local_names(body, &mut names);
        }
    }
    let mut prefix = "_slang_init_".to_owned();
    while names.iter().any(|n| n.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        structures: module
            .structures
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect(),
        prefix,
        functions: vec![],
        generated_structures: vec![],
    };
    for global in &mut module.globals {
        if let Some(value) = &mut global.initializer {
            lowerer.expression(value)?;
        }
    }
    for function in &mut module.functions {
        if let Some(body) = &mut function.body {
            lowerer.statement(body)?;
        }
    }
    module.functions.extend(lowerer.functions);
    module.structures.extend(lowerer.generated_structures);
    Ok(())
}

fn local_names(statement: &Stmt, names: &mut BTreeSet<String>) {
    match &statement.kind {
        StmtKind::Variable(v) => {
            names.insert(v.name.clone());
        }
        StmtKind::Block(body) => {
            for s in body {
                local_names(s, names)
            }
        }
        StmtKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            local_names(then_branch, names);
            if let Some(s) = else_branch {
                local_names(s, names);
            }
        }
        StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => local_names(body, names),
        StmtKind::For {
            initializer, body, ..
        } => {
            local_names(initializer, names);
            local_names(body, names);
        }
        StmtKind::Switch { cases, .. } => {
            for case in cases {
                for s in &case.statements {
                    local_names(s, names)
                }
            }
        }
        _ => {}
    }
}

struct Lowerer {
    structures: BTreeMap<String, Struct>,
    prefix: String,
    functions: Vec<Function>,
    generated_structures: Vec<Struct>,
}

impl Lowerer {
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(body) => {
                for s in body {
                    self.statement(s)?;
                }
            }
            StmtKind::Variable(v) => {
                if let Some(value) = &mut v.initializer {
                    self.expression(value)?;
                }
            }
            StmtKind::Expression(value) | StmtKind::Return(Some(value)) => {
                self.expression(value)?
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression(condition)?;
                self.statement(then_branch)?;
                if let Some(branch) = else_branch {
                    self.statement(branch)?;
                }
            }
            StmtKind::While { condition, body } | StmtKind::DoWhile { condition, body } => {
                self.expression(condition)?;
                self.statement(body)?;
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.statement(initializer)?;
                if let Some(v) = condition {
                    self.expression(v)?;
                }
                if let Some(v) = increment {
                    self.expression(v)?;
                }
                self.statement(body)?;
            }
            StmtKind::Switch { selector, cases } => {
                self.expression(selector)?;
                for case in cases {
                    if let Some(v) = &mut case.value {
                        self.expression(v)?;
                    }
                    for s in &mut case.statements {
                        self.statement(s)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        match &mut expression.kind {
            ExprKind::Construct(values)
            | ExprKind::Call {
                arguments: values, ..
            } => {
                for v in values {
                    self.expression(v)?;
                }
            }
            ExprKind::MethodCall {
                receiver,
                arguments,
                ..
            } => {
                self.expression(receiver)?;
                for v in arguments {
                    self.expression(v)?;
                }
            }
            ExprKind::DeclarationDefault(value)
            | ExprKind::Unary { value, .. }
            | ExprKind::Cast { value }
            | ExprKind::Postfix { value, .. } => self.expression(value)?,
            ExprKind::Binary { left, right, .. } => {
                self.expression(left)?;
                self.expression(right)?;
            }
            ExprKind::Member { base, .. } => self.expression(base)?,
            ExprKind::Index { base, index } => {
                self.expression(base)?;
                self.expression(index)?;
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.expression(condition)?;
                self.expression(then_value)?;
                self.expression(else_value)?;
            }
            _ => {}
        }
        if self.declaration_default(expression) {
            return Ok(());
        }
        if let ExprKind::Construct(values) = &expression.kind
            && matches!(expression.ty, Type::Struct(_) | Type::Array(..))
        {
            // HLSL cannot return an array directly. A private wrapper provides
            // an ordinary value expression for calls, assignments, and globals.
            let array = matches!(expression.ty, Type::Array(..));
            let structure = if let Type::Struct(name) = &expression.ty {
                self.structures[name].clone()
            } else {
                let structure = Struct {
                    name: format!("{}array_{}", self.prefix, self.generated_structures.len()),
                    fields: vec![Variable {
                        name: "_value".into(),
                        ty: expression.ty.clone(),
                        modifiers: vec![],
                        semantic: None,
                        initializer: None,
                        span: expression.span,
                    }],
                };
                self.generated_structures.push(structure.clone());
                structure
            };
            let values = if array {
                vec![expression.clone()]
            } else {
                values.clone()
            };
            if structure.fields.len() != values.len() {
                return Err(Diagnostic::error(
                    "I003",
                    expression.span,
                    "invalid normalized struct initialization",
                ));
            }
            let result = Expr {
                ty: Type::Struct(structure.name.clone()),
                kind: ExprKind::Name("_result".into()),
                span: expression.span,
            };
            let mut parameters = vec![];
            let mut arguments = vec![];
            let mut statements = vec![Stmt {
                span: expression.span,
                kind: StmtKind::Variable(Variable {
                    name: "_result".into(),
                    ty: result.ty.clone(),
                    modifiers: vec![],
                    semantic: None,
                    initializer: Some(Expr {
                        ty: result.ty.clone(),
                        kind: ExprKind::Zero,
                        span: expression.span,
                    }),
                    span: expression.span,
                }),
            }];
            for (field, value) in structure.fields.iter().zip(&values) {
                let path = Expr {
                    ty: field.ty.clone(),
                    kind: ExprKind::Member {
                        base: Box::new(result.clone()),
                        name: field.name.clone(),
                    },
                    span: expression.span,
                };
                Self::field(
                    path,
                    value,
                    &mut parameters,
                    &mut arguments,
                    &mut statements,
                )?;
            }
            let return_type = result.ty.clone();
            statements.push(Stmt {
                span: expression.span,
                kind: StmtKind::Return(Some(result)),
            });
            let name = format!("{}{}", self.prefix, self.functions.len());
            self.functions.push(Function {
                name: name.clone(),
                return_type: return_type.clone(),
                parameters,
                modifiers: vec![],
                semantic: None,
                body: Some(Stmt {
                    kind: StmtKind::Block(statements),
                    span: expression.span,
                }),
                span: expression.span,
            });
            expression.kind = if array {
                ExprKind::Member {
                    base: Box::new(Expr {
                        ty: return_type,
                        kind: ExprKind::Call { name, arguments },
                        span: expression.span,
                    }),
                    name: "_value".into(),
                }
            } else {
                ExprKind::Call { name, arguments }
            };
        }
        Ok(())
    }

    fn field(
        path: Expr,
        value: &Expr,
        parameters: &mut Vec<Variable>,
        arguments: &mut Vec<Expr>,
        statements: &mut Vec<Stmt>,
    ) -> Result<(), Diagnostic> {
        if matches!(value.kind, ExprKind::Zero) {
            return Ok(());
        }
        if let (Type::Array(element, length), ExprKind::Construct(values)) = (&path.ty, &value.kind)
        {
            if values.len() != *length as usize {
                return Err(Diagnostic::error(
                    "I003",
                    value.span,
                    "invalid normalized array initialization",
                ));
            }
            for (index, value) in values.iter().enumerate() {
                let child = Expr {
                    ty: *element.clone(),
                    kind: ExprKind::Index {
                        base: Box::new(path.clone()),
                        index: Box::new(Expr {
                            ty: Type::Scalar(Scalar::Uint),
                            kind: ExprKind::Integer(format!("{index}u")),
                            span: value.span,
                        }),
                    },
                    span: value.span,
                };
                Self::field(child, value, parameters, arguments, statements)?;
            }
            return Ok(());
        }
        let name = format!("_arg_{}", parameters.len());
        parameters.push(Variable {
            name: name.clone(),
            ty: value.ty.clone(),
            modifiers: vec![],
            semantic: None,
            initializer: None,
            span: value.span,
        });
        arguments.push(value.clone());
        statements.push(Stmt {
            span: value.span,
            kind: StmtKind::Expression(Expr {
                ty: path.ty.clone(),
                span: value.span,
                kind: ExprKind::Binary {
                    op: "=".into(),
                    left: Box::new(path),
                    right: Box::new(Expr {
                        ty: value.ty.clone(),
                        kind: ExprKind::Name(name),
                        span: value.span,
                    }),
                },
            }),
        });
        Ok(())
    }
}
