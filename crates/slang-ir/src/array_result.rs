use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_sema::Type;
use slang_source::Diagnostic;
use std::collections::BTreeMap;

/// Preserve source array-valued calls using ordinary HLSL struct return values.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_array_result_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut types = BTreeMap::new();
    let mut signatures = BTreeMap::new();
    let mut return_types = vec![];
    let mut structures = vec![];
    let mut majors = BTreeMap::new();
    for function in &module.functions {
        let major = function
            .modifiers
            .iter()
            .rev()
            .find(|m| matches!(m.as_str(), "row_major" | "column_major"))
            .cloned();
        let signature = (function.name.clone(), function.return_type.to_string());
        if function.body.is_some() || !majors.contains_key(&signature) {
            majors.insert(signature, major);
        }
    }
    for function in &mut module.functions {
        if !matches!(function.return_type, Type::Array(..)) || function.return_type.is_empty_array()
        {
            return_types.push(None);
            continue;
        }
        let signature = (function.name.clone(), function.return_type.to_string());
        let major = majors[&signature].clone();
        let key = format!("{} {major:?}", function.return_type);
        let result = types
            .entry(key)
            .or_insert_with(|| {
                let name = format!("{prefix}{}", structures.len());
                structures.push(Struct {
                    name: name.clone(),
                    fields: vec![Variable {
                        name: "_value".into(),
                        ty: function.return_type.clone(),
                        modifiers: major.into_iter().collect(),
                        semantic: None,
                        binding: None,
                        uniform_layout: None,
                        pack_offset: None,
                        initializer: None,
                        span: function.span,
                    }],
                });
                Type::Struct(name)
            })
            .clone();
        signatures.insert(signature, result.clone());
        return_types.push(Some(result.clone()));
        function.return_type = result;
        function
            .modifiers
            .retain(|m| !matches!(m.as_str(), "row_major" | "column_major"));
    }
    if signatures.is_empty() {
        return Ok(());
    }
    let mut lowerer = Lowerer {
        signatures,
        return_type: None,
    };
    for global in &mut module.globals {
        if let Some(initializer) = &mut global.initializer {
            lowerer.expression(initializer)?;
        }
    }
    for (function, return_type) in module.functions.iter_mut().zip(return_types) {
        lowerer.return_type = return_type;
        if let Some(body) = &mut function.body {
            lowerer.statement(body)?;
        }
    }
    module.structures.extend(structures);
    crate::initialization::lower(module)
}

struct Lowerer {
    signatures: BTreeMap<(String, String), Type>,
    return_type: Option<Type>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        if let ExprKind::Call { name, .. } = &expression.kind
            && let Some(result) = self
                .signatures
                .get(&(name.clone(), expression.ty.to_string()))
        {
            let mut call = expression.clone();
            call.ty = result.clone();
            expression.kind = ExprKind::Member {
                base: Box::new(call),
                name: "_value".into(),
            };
        }
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        if let StmtKind::Return(Some(value)) = &mut statement.kind {
            self.expression(value)?;
            if let Some(result) = &self.return_type {
                *value = Expr {
                    ty: result.clone(),
                    span: value.span,
                    kind: ExprKind::Construct(vec![value.clone()]),
                };
            }
            return Ok(());
        }
        walk_statement(self, statement)
    }
}
