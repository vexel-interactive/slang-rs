//! Keep generic overload declarations separate through concrete call selection.

use super::*;

impl Specializer {
    pub(super) fn register_functions(
        &mut self,
        declarations: Vec<Declaration>,
        known: Option<&BTreeSet<String>>,
    ) -> Result<(), Diagnostic> {
        let mut signatures = BTreeMap::new();
        for mut declaration in declarations {
            let Declaration::Function(function) = &mut declaration else {
                unreachable!()
            };
            let source = function.name.clone();
            let mut names = BTreeSet::new();
            for parameter in &function.generic_parameters {
                if !names.insert(parameter.name.clone()) {
                    return Err(Diagnostic::error(
                        "T003",
                        parameter.span,
                        "duplicate generic parameter",
                    ));
                }
            }
            self.shapes.current_module = self
                .symbols
                .get(&source)
                .map_or(Default::default(), |o| o.module);
            let mut types = vec![];
            for parameter in &function.parameters {
                let mut ty = parameter.ty.clone();
                self.signature_type(&mut ty, &names, 0)?;
                types.push(canonical::type_key(&ty));
            }
            let signature = (source.clone(), function.generic_parameters.len(), types);
            let key = if let Some(key) = signatures.get(&signature) {
                let key: &String = key;
                key.clone()
            } else {
                let key = format!("{}function_template_{}", self.prefix, self.templates.len());
                self.templates.insert(
                    key.clone(),
                    Template {
                        name: source.clone(),
                        function: true,
                        parameters: function.generic_parameters.clone(),
                        declarations: vec![],
                        nested: vec![],
                        statics: vec![],
                        owner: None,
                    },
                );
                self.function_templates
                    .entry(source.clone())
                    .or_default()
                    .push(key.clone());
                signatures.insert(signature, key.clone());
                key
            };
            if let Some(known) = known {
                Validator {
                    known,
                    symbols: &self.symbols,
                    module: self.shapes.current_module.0,
                }
                .declaration(&mut declaration)?;
            }
            self.templates
                .get_mut(&key)
                .unwrap()
                .declarations
                .push(declaration);
        }
        Ok(())
    }

    pub(super) fn signature_type(
        &mut self,
        ty: &mut TypeRef,
        names: &BTreeSet<String>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if depth >= 32 {
            return Err(error(
                ty.span,
                "cyclic or excessively nested generic signature",
            ));
        }
        if let Some(base) = &ty.base
            && let Some(owner) = self.templates.get(&base.name).cloned()
            && let Some(member) = owner
                .nested
                .iter()
                .find(|n| n.owner == owner.name && n.name == ty.name)
            && let Some(Declaration::TypeAlias(alias)) = owner
                .declarations
                .iter()
                .find(|d| declaration_name(d) == member.type_name)
        {
            let mut substitutions = BTreeMap::new();
            for (parameters, arguments) in [
                (&owner.parameters, &base.arguments),
                (&alias.generic_parameters, &ty.arguments),
            ] {
                if arguments.len() > parameters.len() {
                    return Err(error(ty.span, "too many generic type arguments"));
                }
                for (index, parameter) in parameters.iter().enumerate() {
                    let mut argument = match arguments.get(index) {
                        Some(TypeArgument::Type(ty)) => ty.clone(),
                        Some(_) => {
                            return Err(error(
                                ty.span,
                                "generic type parameter requires a type argument",
                            ));
                        }
                        None => parameter
                            .default
                            .clone()
                            .ok_or_else(|| error(ty.span, "missing generic type argument"))?,
                    };
                    substitute::replace_type(&mut argument, &substitutions);
                    substitutions.insert(parameter.name.clone(), argument);
                }
            }
            let mut target = alias.target.clone();
            SignatureSubstitute(&substitutions).ty(&mut target)?;
            target.arrays.splice(0..0, ty.arrays.clone());
            target.matrix_major = ty.matrix_major.or(target.matrix_major);
            target.span = ty.span;
            *ty = target;
            return self.signature_type(ty, names, depth + 1);
        }
        if ty.base.is_none()
            && let Some(template) = self.templates.get(&ty.name).cloned()
            && let Some(Declaration::TypeAlias(alias)) = template.declarations.first()
        {
            let mut substitutions = BTreeMap::new();
            if ty.arguments.len() > template.parameters.len() {
                return Err(error(ty.span, "too many generic type arguments"));
            }
            for (index, parameter) in template.parameters.iter().enumerate() {
                let mut argument = match ty.arguments.get(index) {
                    Some(TypeArgument::Type(ty)) => ty.clone(),
                    Some(_) => {
                        return Err(error(
                            ty.span,
                            "generic type parameter requires a type argument",
                        ));
                    }
                    None => parameter
                        .default
                        .clone()
                        .ok_or_else(|| error(ty.span, "missing generic type argument"))?,
                };
                substitute::replace_type(&mut argument, &substitutions);
                substitutions.insert(parameter.name.clone(), argument);
            }
            let mut target = alias.target.clone();
            SignatureSubstitute(&substitutions).ty(&mut target)?;
            target.arrays.splice(0..0, ty.arrays.clone());
            *ty = target;
            return self.signature_type(ty, names, depth + 1);
        }
        if ty.base.is_none()
            && let Some(alias) = self.aliases.get(&ty.name).cloned()
        {
            let mut alias = alias;
            alias.arrays.splice(0..0, ty.arrays.clone());
            *ty = alias;
            return self.signature_type(ty, names, depth + 1);
        }
        if !argument_names(std::slice::from_ref(ty))
            .iter()
            .any(|n| names.contains(n))
        {
            return self.canonical(ty, depth);
        }
        if let Some(base) = &mut ty.base {
            self.signature_type(base, names, depth + 1)?;
        }
        for argument in &mut ty.arguments {
            if let TypeArgument::Type(ty) = argument {
                self.signature_type(ty, names, depth + 1)?;
            }
            if let TypeArgument::Expression(expression) = argument
                && let Some(value) = self.shape_value(expression)
            {
                expression.kind = ExprKind::Integer(value.to_string());
            }
        }
        for dimension in &mut ty.arrays {
            if let Some(value) = self.shape_value(dimension) {
                dimension.kind = ExprKind::Integer(value.to_string());
            }
        }
        Ok(())
    }

    pub(super) fn apply_function(
        &mut self,
        name: &str,
        supplied: &[TypeArgument],
        span: Span,
    ) -> Result<String, Diagnostic> {
        let templates = self.function_templates[name].clone();
        let mut arguments = vec![];
        for argument in supplied {
            let TypeArgument::Type(argument) = argument else {
                return Err(error(
                    span,
                    "generic type parameter requires a type argument",
                ));
            };
            let mut argument = argument.clone();
            self.canonical(&mut argument, 0)?;
            self.validate_argument(&mut argument)?;
            arguments.push(TypeArgument::Type(argument));
        }
        let key = (
            name.to_owned(),
            arguments
                .iter()
                .map(|a| match a {
                    TypeArgument::Type(ty) => canonical::type_key(ty),
                    _ => unreachable!(),
                })
                .collect(),
        );
        if let Some(name) = self.function_applications.get(&key) {
            return Ok(name.clone());
        }
        let mut candidates = vec![];
        for template in templates {
            let parameters = &self.templates[&template].parameters;
            if supplied.len() > parameters.len()
                || parameters[supplied.len()..]
                    .iter()
                    .any(|p| p.default.is_none())
            {
                continue;
            }
            let defaults = parameters.len() - supplied.len();
            candidates.push(FunctionCandidateInfo {
                function: self.instantiate(&template, &arguments, span)?,
                default_type_arguments: defaults,
                generic: true,
            });
        }
        if candidates.is_empty() {
            return Err(error(
                span,
                "no generic overload accepts the supplied type arguments",
            ));
        }
        if self.application_output.len() >= 1024 {
            return Err(error(span, "generic specialization limit exceeded"));
        }
        let context = self.next_context;
        self.next_context += 1;
        let target = format!("{}function_call_{context}", self.prefix);
        if let Some(origin) = self.symbols.get(name).cloned() {
            self.symbols.insert(target.clone(), origin);
        }
        self.application_output.push(FunctionApplicationInfo {
            name: target.clone(),
            candidates,
        });
        self.function_applications.insert(key, target.clone());
        Ok(target)
    }
}

struct SignatureSubstitute<'a>(&'a BTreeMap<String, TypeRef>);
impl VisitorMut for SignatureSubstitute<'_> {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        substitute::replace_type(ty, self.0);
        visit::walk_type(self, ty)
    }
}

impl crate::checker::Checker {
    pub(crate) fn declare_function_applications(&mut self, module: &Module) {
        for application in &module.function_applications {
            let mut signatures = vec![];
            for candidate in &application.candidates {
                if let Some(overloads) = self.functions.get(&candidate.function) {
                    signatures.extend(overloads.iter().cloned().map(|mut signature| {
                        signature.default_type_arguments = candidate.default_type_arguments;
                        signature.generic = candidate.generic;
                        signature
                    }));
                }
            }
            self.functions.insert(application.name.clone(), signatures);
        }
    }
}
