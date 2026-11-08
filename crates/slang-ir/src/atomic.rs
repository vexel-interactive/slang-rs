use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_source::Diagnostic;

/// HLSL exchange requires an output even when the Slang source discards it.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_atomic_ignored_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        outputs: vec![],
    };
    lowerer.module(module)?;
    // Private scalar storage is per invocation, including calls in global
    // initializers. Declare it first and keep it out of source reflection.
    module.globals.splice(0..0, lowerer.outputs);
    Ok(())
}

struct Lowerer {
    prefix: String,
    outputs: Vec<Variable>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        if let ExprKind::IntrinsicCall { name, arguments } = &mut expression.kind
            && name == "InterlockedExchange"
            && arguments.len() == 2
        {
            let name = format!("{}{}", self.prefix, self.outputs.len());
            let ty = arguments[0].ty.clone();
            self.outputs.push(Variable {
                name: name.clone(),
                ty: ty.clone(),
                modifiers: vec!["static".into()],
                semantic: None,
                binding: None,
                uniform_layout: None,
                pack_offset: None,
                initializer: None,
                span: expression.span,
            });
            arguments.push(Expr {
                kind: ExprKind::Name(name),
                ty,
                span: expression.span,
            });
        }
        Ok(())
    }
}
