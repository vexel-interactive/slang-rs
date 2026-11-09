use crate::{
    Initialization, InitializationKind, Type,
    checker::{Checker, Signature},
    span_key,
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Expr, Function, Variable};

impl Checker {
    pub(crate) fn select_constructor(
        &self,
        owner: &str,
        arguments: &[Expr],
        span: Span,
    ) -> Result<Option<Signature>, Diagnostic> {
        let Some(name) = self.constructors.get(owner) else {
            return Ok(None);
        };
        let types: Vec<_> = arguments
            .iter()
            .map(|a| {
                self.expression_types
                    .get(&a.id)
                    .cloned()
                    .unwrap_or(Type::Error)
            })
            .collect();
        let overloads = &self.functions[name];
        let target = Type::Struct(owner.into());
        if types == [target.clone()] && !overloads.iter().any(|s| s.parameters == [target.clone()])
        {
            return Ok(None);
        }
        let module = self
            .symbols
            .get(name)
            .map_or(slang_module::ModuleId(0), |o| o.module);
        let mut candidates: Vec<_> = overloads
            .iter()
            .filter(|s| {
                (s.public || module == self.current_module)
                    && types.len() <= s.parameters.len()
                    && s.defaults[types.len()..].iter().all(Option::is_some)
                    && types
                        .iter()
                        .zip(&s.parameters)
                        .all(|(a, b)| *a == Type::InitializerList || a.convertible_to(b))
            })
            .collect();
        let score = |s: &&Signature| -> usize {
            types
                .iter()
                .zip(&s.parameters)
                .zip(arguments)
                .map(|((a, b), e)| {
                    a.literal_conversion_cost(b, slang_syntax::integer_literal(e).ok().flatten())
                })
                .sum()
        };
        candidates.sort_by_key(score);
        let Some(best) = candidates.first() else {
            return Err(Diagnostic::error(
                "T014",
                span,
                format!("no matching constructor for {}", self.source_name(owner)),
            ));
        };
        if candidates.get(1).is_some_and(|s| score(s) == score(best)) {
            return Err(Diagnostic::error(
                "T014",
                span,
                format!("ambiguous constructor for {}", self.source_name(owner)),
            ));
        }
        if !best.defined {
            return Err(Diagnostic::error(
                "T014",
                span,
                "constructor has no definition",
            ));
        }
        Ok(Some((*best).clone()))
    }

    fn automatic_initialization(
        &self,
        ty: &Type,
        span: Span,
    ) -> Result<Option<Initialization>, Diagnostic> {
        match ty {
            Type::Struct(owner) if self.constructors.contains_key(owner) => {
                // A declaration with no matching default constructor remains uninitialized.
                if !self.functions[&self.constructors[owner]]
                    .iter()
                    .any(|s| s.defaults.iter().all(Option::is_some))
                {
                    return Ok(None);
                }
                self.constructor_plan(ty, &[], span)
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn automatic_variable(&mut self, variable: &Variable, local: bool) {
        if variable.initializer.is_some()
            || !local && variable.modifiers.iter().any(|m| m == "const")
        {
            return;
        }
        let ty = self.resolve_type(&variable.ty);
        match self.automatic_initialization(&ty, variable.span) {
            Ok(Some(plan)) => {
                self.variable_initializations
                    .insert(span_key(variable.span), plan);
            }
            Ok(None) => {}
            Err(error) => self.diagnostics.push(error),
        }
    }

    pub(crate) fn prepare_constructor(&mut self, function: &Function) {
        let owner = function.constructor.as_ref().unwrap();
        let fields = self.struct_definitions[owner].fields.clone();
        let mut values = vec![];
        for field in fields {
            let ty = self.structures[owner][&field.name].clone();
            let value = if let Some(value) = &field.initializer {
                Some(Initialization {
                    ty,
                    kind: InitializationKind::DeclarationDefault(Box::new(value.clone())),
                })
            } else {
                match self.automatic_initialization(&ty, field.span) {
                    Ok(value) => value,
                    Err(error) => {
                        self.diagnostics.push(error);
                        None
                    }
                }
            };
            if let Some(value) = value {
                values.push((field.name, value));
            }
        }
        self.constructor_initializations
            .insert(span_key(function.span), values);
    }
}
