use crate::*;
use slang_sema::{CheckedModule, Type, span_key};
use slang_source::{Diagnostic, Span};
use slang_syntax as ast;

pub fn lower(checked: &CheckedModule) -> Result<Module, Diagnostic> {
    let lowerer = Lowerer {
        checked,
        initializing: std::cell::RefCell::new(vec![]),
        initialization_nodes: std::cell::Cell::new(65536),
    };
    let mut module = Module {
        matrix_major: checked.matrix_major,
        structures: vec![],
        constant_buffers: vec![],
        globals: vec![],
        global_uniform_binding: checked.global_uniform_binding,
        global_uniform_size: None,
        global_uniform_fields: None,
        uniform_value_functions: Default::default(),
        uniform_types: Default::default(),
        functions: vec![],
        entry_points: checked.entry_points.clone(),
    };
    let buffer_members: std::collections::BTreeMap<_, _> = checked
        .module
        .constant_buffers
        .iter()
        .flat_map(|b| &b.members)
        .map(|m| (span_key(m.span), m))
        .collect();
    let global_declarations: std::collections::BTreeMap<_, _> = checked
        .module
        .declarations
        .iter()
        .filter_map(|d| match d {
            ast::Declaration::Global(v) => Some((v.name.as_str(), v)),
            _ => None,
        })
        .collect();
    for declaration in &checked.module.declarations {
        match declaration {
            ast::Declaration::TypeAlias(_) => {}
            ast::Declaration::Struct(structure) => module.structures.push(Struct {
                name: structure.name.clone(),
                fields: structure
                    .fields
                    .iter()
                    .map(|f| {
                        let mut field = f.clone();
                        field.initializer = None;
                        lowerer.variable(&field)
                    })
                    .collect::<Result<_, _>>()?,
            }),
            ast::Declaration::Global(variable) => {
                if buffer_members
                    .get(&span_key(variable.span))
                    .is_some_and(|m| !m.static_storage)
                {
                    continue;
                }
                let mut global = lowerer.variable(variable)?;
                if let Some(&value) = checked.global_constants.get(&span_key(variable.span)) {
                    let kind = match global.ty {
                        Type::Scalar(slang_sema::Scalar::Bool) => ExprKind::Bool(value != 0),
                        Type::Scalar(slang_sema::Scalar::Uint) => {
                            ExprKind::Integer(format!("{value}u"))
                        }
                        _ => ExprKind::Integer(value.to_string()),
                    };
                    global.initializer = Some(Expr {
                        ty: global.ty.clone(),
                        kind,
                        span: variable.initializer.as_ref().unwrap().span,
                    });
                }
                module.globals.push(global);
            }
            ast::Declaration::Function(function) => module.functions.push(Function {
                name: checked
                    .function_names
                    .get(&span_key(function.span))
                    .cloned()
                    .unwrap_or_else(|| function.name.clone()),
                return_type: lowerer.declaration_type(function.span)?,
                parameters: function
                    .parameters
                    .iter()
                    .map(|p| {
                        let mut parameter = p.clone();
                        parameter.initializer = None;
                        lowerer.variable(&parameter)
                    })
                    .collect::<Result<_, _>>()?,
                modifiers: lowerer.target_modifiers(&function.modifiers, function.span),
                semantic: function.semantic.clone(),
                body: function
                    .body
                    .as_ref()
                    .map(|b| lowerer.statement(b))
                    .transpose()?,
                span: function.span,
            }),
        }
    }
    for buffer in &checked.module.constant_buffers {
        let mut fields = vec![];
        for member in buffer.members.iter().filter(|m| !m.static_storage) {
            let field = global_declarations
                .get(member.variable_name.as_str())
                .ok_or_else(|| error(member.span))?;
            let mut lowered = lowerer.variable(field)?;
            lowered.pack_offset = member.pack_offset.map(|p| p.offset);
            fields.push(lowered);
        }
        module.constant_buffers.push(ConstantBuffer {
            name: buffer.name.clone(),
            variable_name: buffer.variable_name.clone(),
            fields,
            views: vec![],
        });
    }
    crate::initialization::lower(&mut module)?;
    crate::intrinsic::lower(&mut module)?;
    crate::geometric::lower(&mut module)?;
    crate::derivative::lower(&mut module)?;
    crate::atomic::lower(&mut module)?;
    crate::static_local::lower(&mut module)?;
    crate::scope::lower(&mut module);
    crate::uniform::lower(&mut module, checked)?;
    crate::packed_uniform::lower(&mut module)?;
    crate::matrix_storage::lower(&mut module)?;
    crate::matrix_resource::lower(&mut module)?;
    Ok(module)
}

struct Lowerer<'a> {
    checked: &'a CheckedModule,
    initializing: std::cell::RefCell<Vec<ast::ExprId>>,
    initialization_nodes: std::cell::Cell<usize>,
}

fn error(span: Span) -> Diagnostic {
    Diagnostic::error(
        "I001",
        span,
        "missing or invalid semantic information during IR lowering",
    )
}

impl Lowerer<'_> {
    fn target_modifiers(&self, modifiers: &[String], span: Span) -> Vec<String> {
        let mut result: Vec<_> = modifiers
            .iter()
            .filter(|m| {
                !matches!(
                    m.as_str(),
                    "public" | "internal" | "row_major" | "column_major"
                )
            })
            .cloned()
            .collect();
        if let Some(major) = self.checked.matrix_layouts.get(&span_key(span)) {
            result.push(
                match major {
                    slang_layout::MatrixMajor::Row => "row_major",
                    slang_layout::MatrixMajor::Column => "column_major",
                }
                .into(),
            );
        }
        result
    }

    fn declaration_type(&self, span: Span) -> Result<Type, Diagnostic> {
        self.checked
            .declaration_types
            .get(&span_key(span))
            .filter(|t| **t != Type::Error)
            .cloned()
            .ok_or_else(|| error(span))
    }

    fn variable(&self, variable: &ast::Variable) -> Result<Variable, Diagnostic> {
        Ok(Variable {
            name: variable.name.clone(),
            ty: self.declaration_type(variable.span)?,
            modifiers: self.target_modifiers(&variable.modifiers, variable.span),
            semantic: variable.semantic.clone(),
            binding: self
                .checked
                .resource_bindings
                .get(&span_key(variable.span))
                .copied(),
            uniform_layout: None,
            pack_offset: variable.pack_offset.map(|p| p.offset),
            initializer: variable
                .initializer
                .as_ref()
                .map(|e| self.expression(e))
                .transpose()?,
            span: variable.span,
        })
    }

    fn statement(&self, statement: &ast::Stmt) -> Result<Stmt, Diagnostic> {
        let kind = match &statement.kind {
            ast::StmtKind::Block(statements) => StmtKind::Block(
                statements
                    .iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
            ),
            ast::StmtKind::Variable(variable) => StmtKind::Variable(self.variable(variable)?),
            ast::StmtKind::Variables(variables) => StmtKind::Variables(
                variables
                    .iter()
                    .map(|v| self.variable(v))
                    .collect::<Result<_, _>>()?,
            ),
            ast::StmtKind::Expression(expr) => StmtKind::Expression(self.expression(expr)?),
            ast::StmtKind::Return(expr) => {
                StmtKind::Return(expr.as_ref().map(|e| self.expression(e)).transpose()?)
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => StmtKind::If {
                condition: self.expression(condition)?,
                then_branch: Box::new(self.statement(then_branch)?),
                else_branch: else_branch
                    .as_ref()
                    .map(|s| self.statement(s).map(Box::new))
                    .transpose()?,
            },
            ast::StmtKind::While { condition, body } => StmtKind::While {
                condition: self.expression(condition)?,
                body: Box::new(self.statement(body)?),
            },
            ast::StmtKind::DoWhile { body, condition } => StmtKind::DoWhile {
                body: Box::new(self.statement(body)?),
                condition: self.expression(condition)?,
            },
            ast::StmtKind::Switch { selector, cases } => StmtKind::Switch {
                selector: self.expression(selector)?,
                cases: cases
                    .iter()
                    .map(|case| {
                        Ok(SwitchCase {
                            value: case
                                .value
                                .as_ref()
                                .map(|value| {
                                    let constant = self
                                        .checked
                                        .case_values
                                        .get(&value.id)
                                        .ok_or_else(|| error(value.span))?;
                                    let ty = self
                                        .checked
                                        .expression_types
                                        .get(&selector.id)
                                        .cloned()
                                        .ok_or_else(|| error(selector.span))?;
                                    Ok(Expr {
                                        kind: ExprKind::Integer(
                                            if ty == Type::Scalar(slang_sema::Scalar::Uint) {
                                                format!("{constant}u")
                                            } else {
                                                constant.to_string()
                                            },
                                        ),
                                        ty,
                                        span: value.span,
                                    })
                                })
                                .transpose()?,
                            statements: case
                                .statements
                                .iter()
                                .map(|statement| self.statement(statement))
                                .collect::<Result<_, _>>()?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            ast::StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => StmtKind::For {
                initializer: Box::new(self.statement(initializer)?),
                condition: condition.as_ref().map(|e| self.expression(e)).transpose()?,
                increment: increment.as_ref().map(|e| self.expression(e)).transpose()?,
                body: Box::new(self.statement(body)?),
            },
            ast::StmtKind::Break => StmtKind::Break,
            ast::StmtKind::Continue => StmtKind::Continue,
            ast::StmtKind::Discard => StmtKind::Discard,
            ast::StmtKind::Empty => StmtKind::Empty,
        };
        Ok(Stmt {
            kind,
            span: statement.span,
        })
    }

    fn expression(&self, expression: &ast::Expr) -> Result<Expr, Diagnostic> {
        let ty = self
            .checked
            .expression_types
            .get(&expression.id)
            .filter(|t| **t != Type::Error)
            .cloned()
            .ok_or_else(|| error(expression.span))?;
        let kind = match &expression.kind {
            ast::ExprKind::InitializerList(_) => {
                let value = self
                    .checked
                    .initializations
                    .get(&expression.id)
                    .ok_or_else(|| error(expression.span))?;
                return self.planned_initialization(expression.id, value, expression.span);
            }
            ast::ExprKind::Integer(value) => ExprKind::Integer(value.clone()),
            ast::ExprKind::Float(value) => ExprKind::Float(value.clone()),
            ast::ExprKind::Bool(value) => ExprKind::Bool(*value),
            ast::ExprKind::Name(name) => {
                if self.checked.global_references.contains_key(&expression.id) {
                    ExprKind::GlobalName(name.clone())
                } else {
                    ExprKind::Name(name.clone())
                }
            }
            ast::ExprKind::Unary { op, value } => ExprKind::Unary {
                op: op.clone(),
                value: Box::new(self.expression(value)?),
            },
            ast::ExprKind::Cast { value, .. } => ExprKind::Cast {
                value: Box::new(self.expression(value)?),
            },
            ast::ExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: op.clone(),
                left: Box::new(self.expression(left)?),
                right: Box::new(self.expression(right)?),
            },
            ast::ExprKind::Call { callee, arguments } => {
                if let Some(value) = self.checked.initializations.get(&callee.id) {
                    return self.planned_initialization(callee.id, value, expression.span);
                }
                let mut arguments: Vec<_> = arguments
                    .iter()
                    .map(|a| self.expression(a))
                    .collect::<Result<_, _>>()?;
                if let Some(defaults) = self.checked.default_arguments.get(&callee.id) {
                    self.enter_default(callee.id, expression.span)?;
                    let result = defaults
                        .iter()
                        .map(|value| self.initialization(value, expression.span))
                        .collect::<Result<Vec<_>, _>>();
                    self.initializing.borrow_mut().pop();
                    arguments.extend(result?);
                }
                // Intrinsic and resource signatures can infer a wider input type
                // from output parameters. Preserve that choice in target code.
                if !self.checked.call_names.contains_key(&callee.id)
                    && let Some(Type::Callable { parameters, .. }) =
                        self.checked.expression_types.get(&callee.id)
                {
                    for (argument, target) in arguments.iter_mut().zip(parameters) {
                        if &argument.ty != target {
                            *argument = Expr {
                                ty: target.clone(),
                                span: argument.span,
                                kind: ExprKind::Cast {
                                    value: Box::new(argument.clone()),
                                },
                            };
                        }
                    }
                }
                match &callee.kind {
                    ast::ExprKind::TypeApplication(_) => ExprKind::Call {
                        name: self
                            .checked
                            .call_names
                            .get(&callee.id)
                            .cloned()
                            .ok_or_else(|| error(callee.span))?,
                        arguments,
                    },
                    ast::ExprKind::Name(name)
                        if !self.checked.call_names.contains_key(&callee.id) =>
                    {
                        ExprKind::IntrinsicCall {
                            name: name.clone(),
                            arguments,
                        }
                    }
                    ast::ExprKind::Name(name) => ExprKind::Call {
                        name: self
                            .checked
                            .call_names
                            .get(&callee.id)
                            .cloned()
                            .unwrap_or_else(|| name.clone()),
                        arguments,
                    },
                    ast::ExprKind::Member { base, name } => ExprKind::MethodCall {
                        receiver: Box::new(self.expression(base)?),
                        name: name.clone(),
                        arguments,
                    },
                    _ => return Err(error(callee.span)),
                }
            }
            ast::ExprKind::Member { base, name } => ExprKind::Member {
                base: Box::new(self.expression(base)?),
                name: name.clone(),
            },
            ast::ExprKind::Index { base, index } => ExprKind::Index {
                base: Box::new(self.expression(base)?),
                index: Box::new(self.expression(index)?),
            },
            ast::ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => ExprKind::Conditional {
                condition: Box::new(self.expression(condition)?),
                then_value: Box::new(self.expression(then_value)?),
                else_value: Box::new(self.expression(else_value)?),
            },
            ast::ExprKind::Postfix { op, value } => ExprKind::Postfix {
                op: op.clone(),
                value: Box::new(self.expression(value)?),
            },
            ast::ExprKind::String(_) | ast::ExprKind::TypeApplication(_) => {
                return Err(error(expression.span));
            }
        };
        Ok(Expr {
            ty,
            kind,
            span: expression.span,
        })
    }

    fn planned_initialization(
        &self,
        id: ast::ExprId,
        value: &slang_sema::Initialization,
        span: Span,
    ) -> Result<Expr, Diagnostic> {
        self.enter_default(id, span)?;
        let result = self.initialization(value, span);
        self.initializing.borrow_mut().pop();
        result
    }

    fn enter_default(&self, id: ast::ExprId, span: Span) -> Result<(), Diagnostic> {
        if self.initializing.borrow().is_empty() {
            self.initialization_nodes.set(65536);
        }
        if self.initializing.borrow().len() >= 32 || self.initializing.borrow().contains(&id) {
            return Err(Diagnostic::error(
                "I002",
                span,
                "recursive or excessively nested default expansion",
            ));
        }
        self.initializing.borrow_mut().push(id);
        Ok(())
    }

    fn initialization(
        &self,
        value: &slang_sema::Initialization,
        span: Span,
    ) -> Result<Expr, Diagnostic> {
        let remaining = self.initialization_nodes.get();
        if remaining == 0 {
            return Err(Diagnostic::error(
                "I002",
                span,
                "aggregate default expansion size limit exceeded",
            ));
        }
        self.initialization_nodes.set(remaining - 1);
        let kind = match &value.kind {
            slang_sema::InitializationKind::Zero => ExprKind::Zero,
            slang_sema::InitializationKind::Aggregate(values) => ExprKind::Construct(
                values
                    .iter()
                    .map(|v| self.initialization(v, span))
                    .collect::<Result<_, _>>()?,
            ),
            slang_sema::InitializationKind::Value(source)
            | slang_sema::InitializationKind::DeclarationDefault(source) => {
                let expression = self.expression(source)?;
                let expression = if expression.ty == value.ty {
                    expression
                } else {
                    Expr {
                        ty: value.ty.clone(),
                        kind: ExprKind::Cast {
                            value: Box::new(expression),
                        },
                        span,
                    }
                };
                return Ok(
                    if matches!(
                        value.kind,
                        slang_sema::InitializationKind::DeclarationDefault(_)
                    ) {
                        Expr {
                            ty: expression.ty.clone(),
                            span: expression.span,
                            kind: ExprKind::DeclarationDefault(Box::new(expression)),
                        }
                    } else {
                        expression
                    },
                );
            }
        };
        Ok(Expr {
            ty: value.ty.clone(),
            kind,
            span,
        })
    }
}
