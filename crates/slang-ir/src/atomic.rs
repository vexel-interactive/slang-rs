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

        let (arguments, ty) = match &mut expression.kind {
            ExprKind::IntrinsicCall {
                name,
                arguments,
            } if name == "InterlockedExchange" && arguments.len() == 2 => {
                let ty = arguments[0].ty.clone();
                (arguments, ty)
            }
            ExprKind::MethodCall {
                receiver,
                name,
                arguments,
            } if name == "InterlockedExchange"
                && arguments.len() == 2
                && matches!(
                    receiver.ty,
                    slang_sema::Type::ByteBuffer {
                        writable: true
                    }
                ) =>
            {
                (arguments, slang_sema::Type::Scalar(slang_sema::Scalar::Uint))
            }
            _ => return Ok(()),
        };
        let name = format!("{}{}", self.prefix, self.outputs.len());
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
            kind: ExprKind::GlobalName(name),
            ty,
            span: expression.span,
        });
        Ok(())
    }
}
