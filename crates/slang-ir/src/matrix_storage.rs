use crate::{
    visit::{Visitor, walk_statement},
    *,
};
use slang_sema::Type;
use slang_source::{Diagnostic, Span};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Load structured-buffer matrix arrays as logical values. DXC's aggregate
/// copies otherwise carry the buffer's matrix strides into ordinary arrays.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let mut affected = BTreeSet::new();
    let mut reverse: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut queue = VecDeque::new();
    for structure in &module.structures {
        for field in &structure.fields {
            let mut element = &field.ty;
            while let Type::Array(inner, _) = element {
                element = inner;
            }
            if let Type::Struct(name) = element {
                reverse
                    .entry(name.clone())
                    .or_default()
                    .push(structure.name.clone());
            }
            let major = field
                .modifiers
                .iter()
                .rev()
                .find_map(|m| match m.as_str() {
                    "row_major" => Some(slang_layout::MatrixMajor::Row),
                    "column_major" => Some(slang_layout::MatrixMajor::Column),
                    _ => None,
                })
                .unwrap_or(module.matrix_major);
            if matches!(field.ty, Type::Array(..))
                && matches!(element, Type::Matrix(..))
                && major != module.matrix_major
                && affected.insert(structure.name.clone())
            {
                queue.push_back(structure.name.clone());
            }
        }
    }
    while let Some(name) = queue.pop_front() {
        if let Some(parents) = reverse.get(&name) {
            for parent in parents {
                if affected.insert(parent.clone()) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }
    if affected.is_empty() {
        return Ok(());
    }
    let names = crate::visit::names(module);
    let mut prefix = "_slang_matrix_storage_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        structures: module
            .structures
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect(),
        affected,
        outputs: module
            .functions
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    f.parameters
                        .iter()
                        .map(|p| {
                            p.modifiers
                                .iter()
                                .any(|m| matches!(m.as_str(), "out" | "inout"))
                        })
                        .collect(),
                )
            })
            .collect(),
        prefix,
        cache: BTreeMap::new(),
        helpers: vec![],
        remaining: 65536,
    };
    lowerer.module(module)?;
    module.functions.extend(lowerer.helpers);
    Ok(())
}

struct Lowerer {
    structures: BTreeMap<String, Struct>,
    affected: BTreeSet<String>,
    outputs: BTreeMap<String, Vec<bool>>,
    prefix: String,
    cache: BTreeMap<(String, String), String>,
    helpers: Vec<Function>,
    remaining: usize,
}

impl Lowerer {
    fn transform(&mut self, expression: &mut Expr, location: bool) -> Result<(), Diagnostic> {
        match &mut expression.kind {
            ExprKind::Construct(values) => {
                for value in values {
                    self.transform(value, false)?;
                }
            }
            ExprKind::Call { name, arguments } => {
                let outputs = self.outputs.get(name).cloned().unwrap_or_default();
                for (index, value) in arguments.iter_mut().enumerate() {
                    self.transform(value, outputs.get(index).copied().unwrap_or(false))?;
                }
            }
            ExprKind::IntrinsicCall { name, arguments } => {
                let atomic = name.starts_with("Interlocked");
                let last = arguments.len().saturating_sub(1);
                for (index, value) in arguments.iter_mut().enumerate() {
                    self.transform(value, atomic && (index == 0 || index == last))?;
                }
            }
            ExprKind::MethodCall {
                receiver,
                name,
                arguments,
            } => {
                self.transform(receiver, true)?;
                let last = arguments.len().saturating_sub(1);
                for (index, value) in arguments.iter_mut().enumerate() {
                    self.transform(
                        value,
                        name == "GetDimensions"
                            || (name.starts_with("Interlocked") && index == last),
                    )?;
                }
            }
            ExprKind::DeclarationDefault(value) | ExprKind::Cast { value } => {
                self.transform(value, false)?
            }
            ExprKind::Unary { op, value } => {
                self.transform(value, matches!(op.as_str(), "++" | "--"))?
            }
            ExprKind::Postfix { value, .. } => self.transform(value, true)?,
            ExprKind::Binary { op, left, right } => {
                let assignment = matches!(
                    op.as_str(),
                    "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                );
                self.transform(left, assignment)?;
                self.transform(right, false)?;
            }
            ExprKind::Member { base, .. } => self.transform(base, location)?,
            ExprKind::Index { base, index } => {
                self.transform(base, location)?;
                self.transform(index, false)?;
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.transform(condition, false)?;
                self.transform(then_value, location)?;
                self.transform(else_value, location)?;
            }
            _ => {}
        }
        if location {
            return Ok(());
        }
        let (base, index) = match &expression.kind {
            ExprKind::Index { base, index } => (base.as_ref(), index.as_ref()),
            ExprKind::MethodCall {
                receiver,
                name,
                arguments,
            } if name == "Load" && arguments.len() == 1 => (receiver.as_ref(), &arguments[0]),
            _ => return Ok(()),
        };
        let Type::Buffer { element, .. } = &base.ty else {
            return Ok(());
        };
        let Type::Struct(name) = element.as_ref() else {
            return Ok(());
        };
        if !self.affected.contains(name) {
            return Ok(());
        }
        let helper = self.helper(&base.ty, &index.ty, expression.span)?;
        expression.kind = ExprKind::Call {
            name: helper,
            arguments: vec![base.clone(), index.clone()],
        };
        Ok(())
    }

    fn helper(&mut self, buffer: &Type, index: &Type, span: Span) -> Result<String, Diagnostic> {
        let key = (buffer.to_string(), index.to_string());
        if let Some(name) = self.cache.get(&key) {
            return Ok(name.clone());
        }
        let name = format!("{}{}", self.prefix, self.helpers.len());
        let Type::Buffer { element, .. } = buffer else {
            unreachable!()
        };
        let source = Expr {
            ty: *element.clone(),
            span,
            kind: ExprKind::Index {
                base: Box::new(Expr {
                    ty: buffer.clone(),
                    span,
                    kind: ExprKind::Name("_buffer".into()),
                }),
                index: Box::new(Expr {
                    ty: index.clone(),
                    span,
                    kind: ExprKind::Name("_index".into()),
                }),
            },
        };
        let initializer = self.value(source)?;
        self.helpers.push(Function {
            name: name.clone(),
            return_type: *element.clone(),
            parameters: vec![
                variable("_buffer", buffer.clone(), None, span),
                variable("_index", index.clone(), None, span),
            ],
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
                            *element.clone(),
                            Some(initializer),
                            span,
                        )),
                    },
                    Stmt {
                        span,
                        kind: StmtKind::Return(Some(Expr {
                            ty: *element.clone(),
                            span,
                            kind: ExprKind::Name("_result".into()),
                        })),
                    },
                ]),
            }),
        });
        self.cache.insert(key, name.clone());
        Ok(name)
    }

    fn value(&mut self, source: Expr) -> Result<Expr, Diagnostic> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| error(source.span))?;
        let span = source.span;
        let values = match &source.ty {
            Type::Struct(name) if self.affected.contains(name) => {
                let structure = self.structures[name].clone();
                let mut values = vec![];
                for field in structure.fields {
                    values.push(self.value(Expr {
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
            Type::Array(element, count) => {
                if u64::from(*count) > self.remaining as u64 {
                    return Err(error(span));
                }
                let mut values = vec![];
                for index in 0..*count {
                    values.push(self.value(Expr {
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
        self.transform(expression, false)
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
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
        "matrix storage conversion expansion limit exceeded",
    )
}
