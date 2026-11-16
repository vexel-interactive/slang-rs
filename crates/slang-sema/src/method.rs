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
            self.member_owner(base)?
        } else {
            let current = self.current_type.as_ref().or(self.current_static_type.as_ref())?;
            let owner = self.lexical_member_owner(current, name)?;
            let type_receiver = owner != *current || self.lookup("this").is_none();
            (owner, type_receiver)
        };

        let function = self.methods.get(&(owner.clone(), name.into())).cloned()?;
        let type_receiver = type_receiver
            || matches!(
                callee.kind,
                ExprKind::Member {
                    static_lookup: true,
                    ..
                }
            );
        let constrained = self.constraint_call_signature(callee.id, &owner);
        let mut matches: Vec<_> = if let Some(signature) = constrained {
            match signature {
                Ok(signature) => vec![(signature, None)],
                Err(()) => return Some(Type::Error),
            }
        } else {
            let visible: Vec<_> = self.functions[&function]
                .iter()
                .filter(|signature| {
                    let method = signature.method.as_ref().unwrap();
                    (!type_receiver || method.is_static)
                        && (signature.public
                            || signature.module == self.current_module
                            || self.constraint_witness_access(signature, &owner, span))
                })
                .cloned()
                .collect();
            let viable = |signature: &Signature| {
                let offset = usize::from(!signature.method.as_ref().unwrap().is_static);
                let parameters = &signature.parameters[offset..];
                types.len() <= parameters.len()
                    && signature.defaults[offset + types.len()..].iter().all(Option::is_some)
                    && types
                        .iter()
                        .zip(parameters)
                        .all(|(a, b)| *a == Type::InitializerList || a.convertible_to(b))
            };
            let mut matches: Vec<_> = visible
                .iter()
                .filter(|signature| viable(signature))
                .cloned()
                .map(|signature| (signature, None))
                .collect();
            matches.extend(
                self.projected_interface_methods(&owner, name, &visible, type_receiver, span)
                    .into_iter()
                    .filter(|(signature, _)| viable(signature))
                    .map(|(signature, binding)| (signature, Some(binding))),
            );
            matches
        };

        let score = |signature: &Signature| {
            let offset = usize::from(!signature.method.as_ref().unwrap().is_static);
            let conversions = types
                .iter()
                .zip(&signature.parameters[offset..])
                .zip(arguments)
                .map(|((a, b), expr)| {
                    a.literal_conversion_cost(b, slang_syntax::integer_literal(expr).ok().flatten())
                })
                .sum::<usize>();
            (conversions, signature.default_type_arguments, signature.generic)
        };

        matches.sort_by_key(|(signature, _)| score(signature));
        let Some((best, binding)) = matches.first() else {
            self.error(
                "T014",
                span,
                format!("no matching method for {}.{name}", self.source_name(&owner)),
            );
            return Some(Type::Error);
        };

        if matches.get(1).is_some_and(|(s, _)| score(s) == score(best)) {
            self.error(
                "T014",
                span,
                format!("ambiguous method '{}.{name}'", self.source_name(&owner)),
            );
            return Some(Type::Error);
        }

        if !best.defined {
            self.error("T014", span, "method has no definition");
        }

        if let Some(binding) = binding {
            self.constraint_calls.insert(callee.id, binding.clone());
            self.check_constraint_defaults(binding);
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
