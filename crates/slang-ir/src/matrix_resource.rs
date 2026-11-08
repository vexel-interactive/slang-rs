use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_layout::MatrixMajor;
use slang_sema::Type;
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// HLSL cannot qualify a generic matrix argument. Use a one-field storage
/// structure and expose its matrix value at each indexed or Load access.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_matrix_element_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        cache: BTreeMap::new(),
        structures: vec![],
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
    Ok(())
}

struct Lowerer {
    prefix: String,
    cache: BTreeMap<String, String>,
    structures: Vec<Struct>,
}
impl Lowerer {
    fn element(&mut self, ty: &Type, major: MatrixMajor, span: Span) -> String {
        let key = format!("{major:?} {ty}");
        if let Some(name) = self.cache.get(&key) {
            return name.clone();
        }
        let name = format!("{}{}", self.prefix, self.structures.len());
        self.structures.push(Struct {
            name: name.clone(),
            fields: vec![Variable {
                name: "_value".into(),
                ty: ty.clone(),
                span,
                modifiers: vec![
                    match major {
                        MatrixMajor::Row => "row_major",
                        MatrixMajor::Column => "column_major",
                    }
                    .into(),
                ],
                semantic: None,
                binding: None,
                uniform_layout: None,
                pack_offset: None,
                initializer: None,
            }],
        });
        self.cache.insert(key, name.clone());
        name
    }
    fn ty(&mut self, ty: &mut Type, span: Span) {
        match ty {
            Type::Buffer {
                element,
                matrix_major,
                ..
            } => {
                if let Some(major) = matrix_major.take() {
                    **element = Type::Struct(self.element(element, major, span));
                } else {
                    self.ty(element, span);
                }
            }
            Type::Array(element, _)
            | Type::ConstantBuffer(element)
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
}
impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        let source = match &expression.kind {
            ExprKind::Index { base, .. } => Some(&base.ty),
            ExprKind::MethodCall { receiver, name, .. } if name == "Load" => Some(&receiver.ty),
            _ => None,
        };
        let storage = if let Some(Type::Buffer {
            element,
            matrix_major: Some(major),
            ..
        }) = source
        {
            Some(self.element(element, *major, expression.span))
        } else {
            None
        };
        walk_expression(self, expression)?;
        self.ty(&mut expression.ty, expression.span);
        if let Some(name) = storage {
            let mut access = expression.clone();
            access.ty = Type::Struct(name);
            expression.kind = ExprKind::Member {
                base: Box::new(access),
                name: "_value".into(),
            };
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Variable(variable) => self.ty(&mut variable.ty, variable.span),
            StmtKind::Variables(variables) => {
                for variable in variables {
                    self.ty(&mut variable.ty, variable.span);
                }
            }
            _ => {}
        }
        walk_statement(self, statement)
    }
}
