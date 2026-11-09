use crate::{
    Type,
    checker::{Checker, Signature},
};
use slang_source::Span;
use slang_syntax::{Expr, ExprKind};

#[derive(Clone, Debug)]
pub enum MethodReceiver {
    Static,
    Explicit(Box<Expr>),
    Implicit(String),
}

impl Checker {
    pub(crate) fn member_call(
        &mut self,
        callee: &Expr,
        base: Option<&Expr>,
        name: &str,
        arguments: &[Expr],
        types: &[Type],
        span: Span,
    ) -> Option<Type> {
        let (owner, type_receiver) = if let Some(base) = base {
            if matches!(base.kind, ExprKind::Name(_) | ExprKind::TypeApplication(_))
                && let Ok(Some(Type::Struct(owner))) = self.constructor_target(base)
            {
                self.expression_types
                    .insert(base.id, Type::Meta(Box::new(Type::Struct(owner.clone()))));
                (owner, true)
            } else {
                match self.expression(base) {
                    Type::Struct(owner) => (owner, false),
                    Type::ConstantBuffer { element, .. } => {
                        let Type::Struct(owner) = *element else {
                            return None;
                        };
                        (owner, false)
                    }
                    Type::Meta(ty) => {
                        let Type::Struct(owner) = *ty else {
                            return None;
                        };
                        (owner, true)
                    }
                    _ => return None,
                }
            }
        } else {
            (self.current_type.clone()?, self.lookup("this").is_none())
        };
        let function = self.methods.get(&(owner.clone(), name.into())).cloned()?;
        let module = self
            .symbols
            .get(&function)
            .map_or(slang_module::ModuleId(0), |o| o.module);
        let mut matches: Vec<_> = self.functions[&function]
            .iter()
            .filter(|signature| {
                let method = signature.method.as_ref().unwrap();
                let offset = usize::from(!method.is_static);
                let parameters = &signature.parameters[offset..];
                (!type_receiver || method.is_static)
                    && (signature.public || module == self.current_module)
                    && types.len() <= parameters.len()
                    && signature.defaults[offset + types.len()..]
                        .iter()
                        .all(Option::is_some)
                    && types
                        .iter()
                        .zip(parameters)
                        .all(|(a, b)| *a == Type::InitializerList || a.convertible_to(b))
            })
            .cloned()
            .collect();
        let score = |signature: &Signature| -> usize {
            let offset = usize::from(!signature.method.as_ref().unwrap().is_static);
            types
                .iter()
                .zip(&signature.parameters[offset..])
                .zip(arguments)
                .map(|((a, b), expr)| {
                    a.literal_conversion_cost(b, slang_syntax::integer_literal(expr).ok().flatten())
                })
                .sum()
        };
        matches.sort_by_key(score);
        let Some(best) = matches.first() else {
            self.error(
                "T014",
                span,
                format!("no matching method for {}.{name}", self.source_name(&owner)),
            );
            return Some(Type::Error);
        };
        if matches.get(1).is_some_and(|s| score(s) == score(best)) {
            self.error(
                "T014",
                span,
                format!("ambiguous method '{}.{name}'", self.source_name(&owner)),
            );
        }
        if !best.defined {
            self.error("T014", span, "method has no definition");
        }
        let mut supplied = arguments.to_vec();
        let mut actual = types.to_vec();
        let receiver = if best.method.as_ref().unwrap().is_static {
            MethodReceiver::Static
        } else {
            let value = if let Some(base) = base {
                base.clone()
            } else {
                let mut value = callee.clone();
                value.kind = ExprKind::Name("this".into());
                value
            };
            actual.insert(0, self.expression(&value));
            supplied.insert(0, value);
            if let Some(base) = base {
                MethodReceiver::Explicit(Box::new(base.clone()))
            } else {
                MethodReceiver::Implicit(owner)
            }
        };
        let result = self.finish_user_call(callee, &supplied, &actual, best, span);
        self.method_receivers.insert(callee.id, receiver);
        Some(result)
    }
}
