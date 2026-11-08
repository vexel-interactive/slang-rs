use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_layout::MatrixMajor;
use slang_sema::Type;
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// HLSL ConstantBuffer requires a struct. Keep source element layouts and wrap
/// other data types for storage, exposing their checked implicit value loads.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_cbuffer_value_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        default_major: module.matrix_major,
        cache: BTreeMap::new(),
        loads: BTreeMap::new(),
        source_structures: module
            .structures
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect(),
        structures: vec![],
        functions: vec![],
        remaining: 65536,
    };
    for global in &mut module.globals {
        lowerer.ty(&mut global.ty, global.span);
    }
    for structure in &mut module.structures {
        for field in &mut structure.fields {
            lowerer.ty(&mut field.ty, field.span);
        }
    }
    for function in &mut module.functions {
        lowerer.ty(&mut function.return_type, function.span);
        for parameter in &mut function.parameters {
            lowerer.ty(&mut parameter.ty, parameter.span);
        }
    }
    lowerer.module(module)?;
    module.structures.splice(0..0, lowerer.structures);
    module.functions.extend(lowerer.functions);
    crate::initialization::lower(module)
}

struct Lowerer {
    prefix: String,
    default_major: MatrixMajor,
    cache: BTreeMap<String, String>,
    loads: BTreeMap<String, (String, Type)>,
    source_structures: BTreeMap<String, Struct>,
    structures: Vec<Struct>,
    functions: Vec<Function>,
    remaining: usize,
}
impl Lowerer {
    fn storage(&mut self, element: &Type, major: MatrixMajor, span: Span) -> String {
        let key = format!("{major:?} {element}");
        if let Some(name) = self.cache.get(&key) {
            return name.clone();
        }
        let name = format!("{}storage_{}", self.prefix, self.cache.len());
        let mut field = variable("_value", element.clone(), None, span);
        let mut scalar = element;
        while let Type::Array(inner, _) = scalar {
            scalar = inner;
        }
        if matches!(scalar, Type::Matrix(..)) {
            field.modifiers.push(
                match major {
                    MatrixMajor::Row => "row_major",
                    MatrixMajor::Column => "column_major",
                }
                .into(),
            );
        }
        self.structures.push(Struct {
            name: name.clone(),
            fields: if *element == Type::Void {
                vec![]
            } else {
                vec![field]
            },
        });
        self.cache.insert(key, name.clone());
        name
    }
    fn ty(&mut self, ty: &mut Type, span: Span) {
        match ty {
            Type::ConstantBuffer {
                element,
                matrix_major,
            } if !matches!(element.as_ref(), Type::Struct(_)) => {
                **element = Type::Struct(self.storage(
                    element,
                    matrix_major.unwrap_or(self.default_major),
                    span,
                ));
                *matrix_major = None;
            }
            Type::Array(element, _)
            | Type::Buffer { element, .. }
            | Type::Texture { element, .. }
            | Type::Meta(element) => self.ty(element, span),
            Type::Callable { parameters, result } => {
                for parameter in parameters {
                    self.ty(parameter, span);
                }
                self.ty(result, span);
            }
            _ => {}
        }
    }
    fn read(
        &mut self,
        value: Expr,
        ty: Type,
        numeric: bool,
        copy: bool,
    ) -> Result<Expr, Diagnostic> {
        let span = value.span;
        if ty == Type::Void {
            let key = value.ty.to_string();
            let name = if let Some((name, _)) = self.loads.get(&key) {
                name.clone()
            } else {
                let name = format!("{}load_{}", self.prefix, self.loads.len());
                self.functions.push(Function {
                    name: name.clone(),
                    return_type: Type::Void,
                    parameters: vec![variable("_buffer", value.ty.clone(), None, span)],
                    modifiers: vec![],
                    semantic: None,
                    span,
                    body: Some(Stmt {
                        span,
                        kind: StmtKind::Block(vec![]),
                    }),
                });
                self.loads.insert(key, (name.clone(), Type::Void));
                name
            };
            return Ok(Expr {
                ty,
                span,
                kind: ExprKind::Call {
                    name,
                    arguments: vec![value],
                },
            });
        }
        if !numeric {
            return Ok(if value.ty == ty {
                value
            } else {
                Expr {
                    ty,
                    span,
                    kind: ExprKind::Cast {
                        value: Box::new(value),
                    },
                }
            });
        }
        if copy && matches!(ty, Type::Array(..)) {
            let key = value.ty.to_string();
            let (name, result_ty) = if let Some(signature) = self.loads.get(&key) {
                signature.clone()
            } else {
                let name = format!("{}load_{}", self.prefix, self.loads.len());
                let result_name = format!("{}result_{}", self.prefix, self.loads.len());
                let result_ty = Type::Struct(result_name.clone());
                self.structures.push(Struct {
                    name: result_name,
                    fields: vec![variable("_value", ty.clone(), None, span)],
                });
                let source = Expr {
                    ty: ty.clone(),
                    span,
                    kind: ExprKind::Member {
                        base: Box::new(Expr {
                            ty: value.ty.clone(),
                            span,
                            kind: ExprKind::Name("_buffer".into()),
                        }),
                        name: "_value".into(),
                    },
                };
                let initializer = Expr {
                    ty: result_ty.clone(),
                    span,
                    kind: ExprKind::Construct(vec![self.materialize(source)?]),
                };
                self.functions.push(Function {
                    name: name.clone(),
                    return_type: result_ty.clone(),
                    parameters: vec![variable("_buffer", value.ty.clone(), None, span)],
                    modifiers: vec![],
                    semantic: None,
                    span,
                    body: Some(Stmt {
                        span,
                        kind: StmtKind::Block(vec![
                            Stmt {
                                span,
                                kind: StmtKind::Variable(variable(
                                    "_result",
                                    result_ty.clone(),
                                    Some(initializer),
                                    span,
                                )),
                            },
                            Stmt {
                                span,
                                kind: StmtKind::Return(Some(Expr {
                                    ty: result_ty.clone(),
                                    span,
                                    kind: ExprKind::Name("_result".into()),
                                })),
                            },
                        ]),
                    }),
                });
                self.loads.insert(key, (name.clone(), result_ty.clone()));
                (name, result_ty)
            };
            Ok(Expr {
                ty,
                span,
                kind: ExprKind::Member {
                    base: Box::new(Expr {
                        ty: result_ty,
                        span,
                        kind: ExprKind::Call {
                            name,
                            arguments: vec![value],
                        },
                    }),
                    name: "_value".into(),
                },
            })
        } else {
            Ok(Expr {
                ty,
                span,
                kind: ExprKind::Member {
                    base: Box::new(value),
                    name: "_value".into(),
                },
            })
        }
    }
    fn materialize(&mut self, source: Expr) -> Result<Expr, Diagnostic> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| error(source.span))?;
        let span = source.span;
        let values = match &source.ty {
            Type::Array(element, count) => {
                if u64::from(*count) > self.remaining as u64 {
                    return Err(error(span));
                }
                let mut values = vec![];
                for index in 0..*count {
                    values.push(self.materialize(Expr {
                        ty: *element.clone(),
                        span,
                        kind: ExprKind::Index {
                            base: Box::new(source.clone()),
                            index: Box::new(integer(index, span)),
                        },
                    })?);
                }
                values
            }
            Type::Struct(name) => {
                let structure = self.source_structures[name].clone();
                let mut values = vec![];
                for field in structure.fields {
                    values.push(self.materialize(Expr {
                        ty: field.ty,
                        span,
                        kind: ExprKind::Member {
                            base: Box::new(source.clone()),
                            name: field.name,
                        },
                    })?);
                }
                values
            }
            Type::Matrix(scalar, rows, columns) => {
                let mut values = vec![];
                for row in 0..*rows {
                    for column in 0..*columns {
                        values.push(Expr {
                            ty: Type::Scalar(*scalar),
                            span,
                            kind: ExprKind::Index {
                                base: Box::new(Expr {
                                    ty: Type::Vector(*scalar, *columns),
                                    span,
                                    kind: ExprKind::Index {
                                        base: Box::new(source.clone()),
                                        index: Box::new(integer(u32::from(row), span)),
                                    },
                                }),
                                index: Box::new(integer(u32::from(column), span)),
                            },
                        });
                    }
                }
                self.remaining = self
                    .remaining
                    .checked_sub(values.len())
                    .ok_or_else(|| error(span))?;
                values
            }
            _ => return Ok(source),
        };
        Ok(Expr {
            ty: source.ty,
            span,
            kind: ExprKind::Construct(values),
        })
    }
}
impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::Index { base, .. } = &mut expression.kind
            && matches!(base.ty, Type::Array(..))
            && let ExprKind::ConstantBufferLoad(value) = &base.kind
        {
            let numeric = matches!(&value.ty,Type::ConstantBuffer {element,..} if !matches!(element.as_ref(),Type::Struct(_)));
            let mut value = *value.clone();
            self.expression(&mut value)?;
            **base = self.read(value, base.ty.clone(), numeric, false)?;
        }
        let numeric = if let ExprKind::ConstantBufferLoad(value) = &expression.kind {
            matches!(&value.ty,Type::ConstantBuffer {element,..} if !matches!(element.as_ref(),Type::Struct(_)))
        } else {
            false
        };
        walk_expression(self, expression)?;
        self.ty(&mut expression.ty, expression.span);
        if let ExprKind::ConstantBufferLoad(value) = &expression.kind {
            *expression = self.read(*value.clone(), expression.ty.clone(), numeric, true)?;
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Variable(v) => self.ty(&mut v.ty, v.span),
            StmtKind::Variables(values) => {
                for v in values {
                    self.ty(&mut v.ty, v.span);
                }
            }
            _ => {}
        }
        walk_statement(self, statement)
    }
}
fn variable(name: &str, ty: Type, initializer: Option<Expr>, span: Span) -> Variable {
    Variable {
        name: name.into(),
        ty,
        initializer,
        span,
        modifiers: vec![],
        semantic: None,
        binding: None,
        uniform_layout: None,
        pack_offset: None,
    }
}
fn integer(value: u32, span: Span) -> Expr {
    Expr {
        ty: Type::Scalar(slang_sema::Scalar::Uint),
        span,
        kind: ExprKind::Integer(format!("{value}u")),
    }
}
fn error(span: Span) -> Diagnostic {
    Diagnostic::error(
        "I004",
        span,
        "constant-buffer value expansion limit exceeded",
    )
}
