use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// DXC rejects zero-length array declarations. Legalize them after computing
/// source layouts, preserving distinct array types as distinct empty structures.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_empty_array_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        types: BTreeMap::new(),
        indices: BTreeMap::new(),
        structures: vec![],
        functions: vec![],
        globals: vec![],
        source_structures: module
            .structures
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect(),
    };
    for global in &mut module.globals {
        let empty_resource = global.ty.is_resource() && global.ty.is_empty_array();
        lowerer.variable(global)?;
        if empty_resource {
            global.modifiers = vec!["static".into()];
            global.initializer.get_or_insert_with(|| Expr {
                ty: global.ty.clone(),
                kind: ExprKind::Zero,
                span: global.span,
            });
        }
    }
    for structure in &mut module.structures {
        for field in &mut structure.fields {
            lowerer.variable(field)?;
        }
    }
    for buffer in &mut module.constant_buffers {
        for field in buffer.fields.iter_mut().chain(&mut buffer.views) {
            lowerer.variable(field)?;
        }
    }
    if let Some(fields) = &mut module.global_uniform_fields {
        for field in fields {
            lowerer.variable(field)?;
        }
    }
    for function in &mut module.functions {
        lowerer.ty(&mut function.return_type);
        for parameter in &mut function.parameters {
            lowerer.variable(parameter)?;
        }
        if let Some(body) = &mut function.body {
            lowerer.statement(body)?;
        }
    }
    module.structures.splice(0..0, lowerer.structures);
    module.functions.extend(lowerer.functions);
    module.globals.extend(lowerer.globals);
    Ok(())
}

struct Lowerer {
    prefix: String,
    types: BTreeMap<String, String>,
    indices: BTreeMap<String, (String, String, Type)>,
    structures: Vec<Struct>,
    functions: Vec<Function>,
    globals: Vec<Variable>,
    source_structures: BTreeMap<String, Struct>,
}

impl Lowerer {
    fn ty(&mut self, ty: &mut Type) {
        match ty {
            Type::Array(..) if ty.is_empty_array() => {
                let key = ty.to_string();
                let name = if let Some(name) = self.types.get(&key) {
                    name.clone()
                } else {
                    let name = format!("{}type_{}", self.prefix, self.types.len());
                    self.structures.push(Struct {
                        name: name.clone(),
                        fields: vec![],
                    });
                    self.types.insert(key, name.clone());
                    name
                };
                *ty = Type::Struct(name);
            }
            Type::Array(element, _)
            | Type::ConstantBuffer { element, .. }
            | Type::Buffer { element, .. }
            | Type::Texture { element, .. }
            | Type::Meta(element) => self.ty(element),
            Type::Callable { parameters, result } => {
                for parameter in parameters {
                    self.ty(parameter);
                }
                self.ty(result);
            }
            _ => {}
        }
    }

    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        self.ty(&mut variable.ty);
        let mut element = &variable.ty;
        while let Type::Array(inner, _) = element {
            element = inner;
        }
        if matches!(element, Type::Struct(name) if self.types.values().any(|n| n == name)) {
            variable
                .modifiers
                .retain(|m| !matches!(m.as_str(), "row_major" | "column_major"));
        }
        if let Some(initializer) = &mut variable.initializer {
            self.expression(initializer)?;
        }
        Ok(())
    }

    fn index(&mut self, base: Expr, index: Expr, mut element: Type) -> Result<Expr, Diagnostic> {
        // Empty array chains have no stored elements. Retain operand evaluation
        // and provide private scratch locations for empty values or undefined
        // zero-length accesses, including writes and output parameters.
        // Undefined resource handles need target-specific poison legalization.
        if self.resource_data(&element) {
            return Err(Diagnostic::error(
                "I006",
                index.span,
                "dynamic indexing of empty resource arrays is not implemented",
            ));
        }
        self.ty(&mut element);
        let key = format!("{} {}", base.ty, index.ty);
        let (function, global, storage) = if let Some(names) = self.indices.get(&key) {
            names.clone()
        } else {
            let number = self.indices.len();
            let function = format!("{}index_{number}", self.prefix);
            let global = format!("{}scratch_{number}", self.prefix);
            let storage = Type::Array(Box::new(element.clone()), 1);
            self.globals.push(Variable {
                modifiers: vec!["static".into()],
                ..variable(&global, storage.clone(), base.span)
            });
            self.functions.push(Function {
                name: function.clone(),
                return_type: Type::Scalar(Scalar::Uint),
                parameters: vec![
                    variable("_array", base.ty.clone(), base.span),
                    variable("_index", index.ty.clone(), index.span),
                ],
                modifiers: vec![],
                semantic: None,
                span: base.span,
                body: Some(Stmt {
                    span: base.span,
                    kind: StmtKind::Block(vec![Stmt {
                        span: base.span,
                        kind: StmtKind::Return(Some(Expr {
                            ty: Type::Scalar(Scalar::Uint),
                            kind: ExprKind::Integer("0u".into()),
                            span: base.span,
                        })),
                    }]),
                }),
            });
            self.indices
                .insert(key, (function.clone(), global.clone(), storage.clone()));
            (function, global, storage)
        };
        let span = base.span;
        Ok(Expr {
            ty: element,
            span,
            kind: ExprKind::Index {
                base: Box::new(Expr {
                    ty: storage,
                    kind: ExprKind::GlobalName(global),
                    span,
                }),
                index: Box::new(Expr {
                    ty: Type::Scalar(Scalar::Uint),
                    kind: ExprKind::Call {
                        name: function,
                        arguments: vec![base, index],
                    },
                    span,
                }),
            },
        })
    }
}

impl Lowerer {
    fn resource_data(&self, ty: &Type) -> bool {
        let mut pending = vec![ty];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(ty) = pending.pop() {
            match ty {
                Type::Array(_, 0) => {}
                Type::Array(element, _) => pending.push(element),
                Type::Struct(name) if visited.insert(name) => {
                    if let Some(structure) = self.source_structures.get(name) {
                        pending.extend(structure.fields.iter().map(|f| &f.ty));
                    }
                }
                ty if ty.is_resource() => return true,
                _ => {}
            }
        }
        false
    }
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        let empty_index = matches!(&expression.kind, ExprKind::Index { base, .. }
            if base.ty.is_empty_array());
        walk_expression(self, expression)?;
        if empty_index {
            if let ExprKind::Index { base, index } = &expression.kind {
                *expression = self.index(*base.clone(), *index.clone(), expression.ty.clone())?;
            }
        } else {
            self.ty(&mut expression.ty);
        }
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Variable(variable) => return self.variable(variable),
            StmtKind::Variables(variables) => {
                for variable in variables {
                    self.variable(variable)?;
                }
                return Ok(());
            }
            _ => {}
        }
        walk_statement(self, statement)
    }
}

fn variable(name: &str, ty: Type, span: Span) -> Variable {
    Variable {
        name: name.into(),
        ty,
        modifiers: vec![],
        semantic: None,
        binding: None,
        uniform_layout: None,
        pack_offset: None,
        initializer: None,
        span,
    }
}
