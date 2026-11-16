//! Check interface bodies on opaque self types and instantiate missing witnesses.

use super::*;
use crate::{Type, checker::Checker};

pub(super) struct Context {
    pub(super) owner: String,
    interface: String,
    is_static: bool,
    members: BTreeSet<String>,
}

impl Specializer {
    fn adapter_expression(&mut self, kind: ExprKind, span: Span) -> Expr {
        let mut expression = Expr::integer(ExprId(self.next_expression), 0, span);
        self.next_expression += 1;
        expression.kind = kind;
        expression
    }

    fn clone_adapter(
        &mut self,
        owner: &str,
        interface: &Interface,
        requirement: &Function,
        adaptation: &crate::conformance::SignatureAdaptation,
        names: &BTreeMap<String, String>,
    ) -> Result<Declaration, Diagnostic> {
        let mut adapted = requirement.clone();
        let target = &adaptation.target;
        let method = requirement.method.as_ref();
        let target_method = target.method.as_ref();
        let span = requirement.span;
        // Abstract requirement spans can equal the synthetic receiver span.
        // Keep the generated function and parameter declaration keys distinct.
        for parameter in &mut adapted.parameters {
            if parameter.span == span {
                parameter.span.end = parameter.span.start;
            }
            if requirement.constructor.is_some()
                && !parameter
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "out" || modifier == "inout")
            {
                // An input witness owns a writable copy even when the source
                // requirement marks that input const. Constructor writes stay
                // within the copy and cannot change the caller's input.
                parameter.modifiers.retain(|modifier| modifier != "const");
            }
        }
        let callee = if let (Some(method), Some(target_method)) = (method, target_method) {
            let receiver = self.adapter_expression(
                ExprKind::Name(if !target_method.is_static && method.is_static {
                    requirement.parameters[0].name.clone()
                } else if method.is_static {
                    owner.into()
                } else {
                    "this".into()
                }),
                span,
            );
            self.adapter_expression(
                ExprKind::Member {
                    base: Box::new(receiver),
                    name: method.name.clone(),
                    static_lookup: target_method.is_static,
                },
                span,
            )
        } else {
            // Call the concrete constructor function, keeping the generated
            // witness outside ordinary type-constructor overload lookup.
            self.adapter_expression(ExprKind::Name(target.name.clone()), span)
        };
        let mut arguments = vec![];
        let mut statements = vec![];
        for (index, ((parameter, target), convert)) in requirement
            .parameters
            .iter()
            .zip(&target.parameters)
            .zip(&adaptation.input_conversions)
            .enumerate()
            .skip(usize::from(target_method.is_some_and(|method| !method.is_static)))
        {
            let value = self.adapter_expression(ExprKind::Name(parameter.name.clone()), span);
            let value = if target.modifiers.iter().any(|m| m == "out" || m == "inout") || !convert {
                value
            } else {
                // Typed input copies freeze overload selection and use ordinary
                // initialization for numeric conversions and buffer value loads.
                let mut name = format!("{}adapter_argument_{index}", self.prefix);
                while requirement.parameters.iter().any(|p| p.name == name) {
                    name.push('_');
                }
                statements.push(Stmt {
                    kind: StmtKind::Variable(Box::new(Variable {
                        name: name.clone(),
                        // The adapter belongs to the interface's module. Keep
                        // the selected concrete type, without moving aliases or
                        // shape constants out of their declaration scope.
                        ty: inference::type_reference(
                            &adaptation.parameter_types[index],
                            &BTreeMap::new(),
                            target.ty.span,
                        )
                        .ok_or_else(|| {
                            error(target.ty.span, "invalid interface adapter parameter type")
                        })?,
                        modifiers: vec![],
                        semantic: None,
                        register: None,
                        pack_offset: None,
                        initializer: Some(value),
                        span: target.span,
                    })),
                    span: target.span,
                });
                self.adapter_expression(ExprKind::Name(name), span)
            };
            arguments.push(value);
        }
        let call = self.adapter_expression(
            ExprKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span,
        );
        statements.push(Stmt {
            kind: StmtKind::Return(Some(call)),
            span,
        });
        adapted.body = Some(Stmt {
            kind: StmtKind::Block(statements),
            span,
        });
        let mut declaration = self.clone_default(owner, interface, &adapted, names, None)?;
        self.instance_output.last_mut().unwrap().witness_target = Some(target.span);
        if requirement.body.is_none()
            && let Declaration::Function(function) = &mut declaration
        {
            function.modifiers.retain(|m| m != "override");
        }
        Ok(declaration)
    }

    pub(super) fn capture_default_selections(&mut self, probe: &InferenceProbe) -> bool {
        if !self.symbolic {
            return false;
        }

        let mut changed = false;

        for context in self.default_method_contexts.values() {
            if !self.symbolic_types.contains(&context.owner) {
                continue;
            }

            changed |= self.checked_default_interfaces.insert(context.interface.clone());
            for (id, binding) in &probe.constraint_calls {
                if binding.owner.name == context.owner {
                    let original = self.expression_origins.get(id).copied().unwrap_or(*id);
                    self.default_body_calls
                        .insert((context.interface.clone(), original), binding.clone());
                }
            }

            for (span, binding) in &probe.constraint_variables {
                if binding.owner.name == context.owner {
                    let (source, start, end, _) = *span;
                    self.default_body_variables.insert(
                        (context.interface.clone(), (source, start, end, 0)),
                        binding.clone(),
                    );
                }
            }
        }

        changed
    }

    pub(super) fn normalize_default_requirement(&mut self, callee: &mut Expr) -> bool {
        let Some(context) = self.default_method_contexts.get(&callee.span.context) else {
            return false;
        };

        if let ExprKind::Name(name) = &callee.kind
            && context.members.contains(name)
            && !self.shapes.scopes.iter().skip(1).any(|s| s.contains_key(name))
        {
            let receiver = if context.is_static {
                &context.owner
            } else {
                "this"
            };

            let mut base = callee.clone();
            base.id = ExprId(self.next_expression);
            base.kind = ExprKind::Name(receiver.into());
            self.next_expression += 1;
            callee.kind = ExprKind::Member {
                base: Box::new(base),
                name: name.clone(),
                static_lookup: context.is_static,
            };
        }

        matches!(&callee.kind, ExprKind::Member { base, name, .. }
            if context.members.contains(name)
                && matches!(&base.kind, ExprKind::Name(n) if n == "this" || n == &context.owner))
    }

    fn default_members(&self, interface: &Interface) -> BTreeSet<String> {
        let mut members = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending = vec![interface];

        while let Some(interface) = pending.pop() {
            if !visited.insert(&interface.name) {
                continue;
            }

            members.extend(
                interface
                    .requirements
                    .iter()
                    .filter_map(|f| f.method.as_ref().map(|m| m.name.clone())),
            );

            for base in &interface.bases {
                if let Some(base) = self.shapes.interface_definitions.get(&base.name) {
                    pending.push(base);
                }
            }
        }

        members
    }

    pub(super) fn interface_headers(
        &mut self,
        interface: &mut Interface,
    ) -> Result<(), Diagnostic> {
        self.shapes.current_module =
            self.symbols.get(&interface.name).map_or(Default::default(), |o| o.module);
        self.shapes.current_static_type = Some(interface.name.clone());

        for base in &mut interface.bases {
            self.ty(base)?;
        }

        for requirement in &mut interface.requirements {
            let body = requirement.body.take();
            visit::walk_function(self, requirement)?;
            requirement.body = body;
        }

        Ok(())
    }

    pub(super) fn validate_default_interface(
        &mut self,
        interface: &Interface,
    ) -> Result<(), Diagnostic> {
        if !interface.requirements.iter().any(|f| f.body.is_some())
            || !self.validated_interfaces.insert(interface.name.clone())
        {
            return Ok(());
        }

        let context = self.next_context;
        self.next_context += 1;
        let owner = format!("{}interface_self_{context}", self.prefix);
        let module = self.symbols.get(&interface.name).map_or(Default::default(), |o| o.module);
        self.symbolic_types.insert(owner.clone());
        self.symbolic_parameters.insert(owner.clone(), witness::reference("This", interface.span));
        self.symbols.insert(
            owner.clone(),
            SymbolOrigin {
                module,
                name: "interface self type".into(),
            },
        );
        self.validation_contexts.insert(context);
        self.register(
            vec![Declaration::Struct(Struct {
                name: owner.clone(),
                generic_parameters: vec![],
                bases: vec![witness::reference(&interface.name, interface.span)],
                fields: vec![],
                modifiers: vec!["public".into()],
                span: interface.span.with_context(context),
            })],
            vec![],
            vec![],
            None,
        )?;
        let names = self.default_names(&owner, interface);

        for requirement in &interface.requirements {
            if requirement.body.is_some()
                && requirement.generic_parameters.is_empty()
                && requirement.method.is_some()
            {
                let context = self.next_context;
                self.next_context += 1;
                self.validation_contexts.insert(context);
                let declaration =
                    self.clone_default(&owner, interface, requirement, &names, Some(context))?;
                self.pending.push_back(declaration);
            }
        }

        Ok(())
    }

    fn default_names(&self, owner: &str, interface: &Interface) -> BTreeMap<String, String> {
        let mut names = BTreeMap::from([(interface.name.clone(), owner.into())]);

        for requirement in &interface.requirements {
            if requirement.method.is_some() || requirement.constructor.is_some() {
                let name = requirement.method.as_ref().map_or("__init", |method| &method.name);
                let target = format!(
                    "{}default_{}_{}_{}_{}_{}_{}",
                    self.prefix,
                    owner.len(),
                    owner,
                    interface.name.len(),
                    interface.name,
                    name.len(),
                    name,
                );
                names.insert(requirement.name.clone(), target);
            }
        }

        names
    }

    fn clone_default(
        &mut self,
        owner: &str,
        interface: &Interface,
        requirement: &Function,
        names: &BTreeMap<String, String>,
        validation_context: Option<usize>,
    ) -> Result<Declaration, Diagnostic> {
        let context = validation_context.unwrap_or_else(|| {
            let c = self.next_context;
            self.next_context += 1;
            c
        });
        let mut types =
            BTreeMap::from([("This".into(), witness::reference(owner, requirement.span))]);
        let mut declaration = Declaration::Function(Box::new(requirement.clone()));

        if self.symbolic && validation_context.is_none() {
            SeedBindings {
                specializer: self,
                interface: &interface.name,
                owner,
                types: &mut types,
            }
            .declaration(&mut declaration)?;
        }

        substitute::Substitute {
            root: &interface.name,
            names,
            types: &types,
            context,
            next_expression: &mut self.next_expression,
            remaining_nodes: &mut self.remaining_nodes,
            expression_origins: &mut self.expression_origins,
            constraint_calls: &mut self.constraint_calls,
            constraint_variables: &mut self.constraint_variables,
        }
        .declaration(&mut declaration)?;
        if requirement.constructor.is_some()
            && let Declaration::Function(function) = &mut declaration
        {
            // The concrete constructor returns the initialized owner. A witness
            // is a forwarding function and must not initialize another receiver.
            function.constructor = None;
        }
        let name = declaration_name(&declaration).to_owned();

        if validation_context.is_none()
            && let Declaration::Function(function) = &mut declaration
        {
            function.modifiers.push("override".into());
        }

        let module = self.symbols.get(&interface.name).map_or(Default::default(), |o| o.module);
        self.symbols.insert(
            name.clone(),
            SymbolOrigin {
                module,
                name: format!(
                    "{owner}.{}",
                    requirement.method.as_ref().map_or("__init", |method| &method.name)
                ),
            },
        );
        self.generated.insert(crate::span_key(declaration_span(&declaration)));
        self.default_method_contexts.insert(
            context,
            Context {
                owner: owner.into(),
                interface: interface.name.clone(),
                is_static: requirement.method.as_ref().is_none_or(|method| method.is_static),
                members: self.default_members(interface),
            },
        );
        self.instance_output.push(GenericInstanceInfo {
            witness_target: None,
            context,
            type_name: name,
            template_name: requirement.name.clone(),
            parameter_count: 0,
            arguments: vec![
                witness::reference(owner, requirement.span),
                witness::reference(&interface.name, requirement.span),
            ],
        });
        Ok(declaration)
    }

    pub(super) fn synthesize_default_methods(
        &mut self,
        module: &Module,
        major: slang_layout::MatrixMajor,
    ) -> Result<bool, Diagnostic> {
        if !module.declarations.iter().any(|d| matches!(d, Declaration::Interface(_))) {
            return Ok(false);
        }

        let mut checker = Checker::new(module, self.symbols.clone(), major);
        checker.symbolic_types = self.symbolic_types.clone();
        checker.declare(module);
        checker.declare_conformances(module);
        let mut candidates =
            BTreeMap::<(String, String, Vec<String>, bool), Vec<(String, Function)>>::new();
        for structure in checker.struct_definitions.values().cloned().collect::<Vec<_>>() {
            if checker.symbolic_types.contains(&structure.name) {
                continue;
            }

            let interfaces = checker.conformances.get(&structure.name).cloned().unwrap_or_default();
            for name in interfaces {
                let interface = checker.interface_definitions[&name].clone();

                for requirement in &interface.requirements {
                    if !requirement.generic_parameters.is_empty()
                        || (requirement.method.is_none() && requirement.constructor.is_none())
                        || (self.symbolic
                            && requirement.body.is_some()
                            && !self.checked_default_interfaces.contains(&name))
                        || checker.direct_witnesses.contains_key(&crate::conformance::witness::key(
                            &structure.name,
                            &name,
                            requirement.span,
                        ))
                        || checker
                            .specialized_signature(module, &structure.name, &name, requirement)
                            .is_some()
                    {
                        continue;
                    }

                    if let Some(adaptation) =
                        checker.signature_adaptation(module, &structure.name, requirement)?
                    {
                        let names = self.default_names(&structure.name, &interface);
                        let declaration = self.clone_adapter(
                            &structure.name,
                            &interface,
                            requirement,
                            &adaptation,
                            &names,
                        )?;
                        self.pending.push_back(declaration);
                        continue;
                    }
                    if requirement.body.is_none() {
                        continue;
                    }

                    let (_, parameters, _) =
                        checker.requirement_signature(&structure.name, requirement);
                    let method = requirement.method.as_ref().unwrap();
                    let key = (
                        structure.name.clone(),
                        method.name.clone(),
                        parameters.iter().map(Type::to_string).collect(),
                        method.is_static,
                    );
                    candidates.entry(key).or_default().push((name.clone(), requirement.clone()));
                }
            }
        }

        let changed = !candidates.is_empty() || !self.pending.is_empty();

        for ((owner, _, _, _), requirements) in candidates {
            for (name, requirement) in requirements {
                let interface = &checker.interface_definitions[&name];
                let names = self.default_names(&owner, interface);
                let declaration =
                    self.clone_default(&owner, interface, &requirement, &names, None)?;
                self.pending.push_back(declaration);
            }
        }

        Ok(changed)
    }
}

struct SeedBindings<'a> {
    specializer: &'a mut Specializer,
    interface: &'a str,
    owner: &'a str,
    types: &'a mut BTreeMap<String, TypeRef>,
}

impl SeedBindings<'_> {
    fn owner_type(&mut self, binding: &ConstraintCallInfo) {
        self.types
            .insert(binding.owner.name.clone(), witness::reference(self.owner, binding.owner.span));
    }
}

impl VisitorMut for SeedBindings<'_> {
    fn expression_id(&mut self, id: &mut ExprId) {
        let original = self.specializer.expression_origins.get(id).copied().unwrap_or(*id);
        if let Some(binding) =
            self.specializer.default_body_calls.get(&(self.interface.into(), original)).cloned()
        {
            self.owner_type(&binding);
            self.specializer.constraint_calls.insert(*id, binding);
        }
    }

    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        let span = crate::span_key(variable.span.with_context(0));

        if let Some(binding) =
            self.specializer.default_body_variables.get(&(self.interface.into(), span)).cloned()
        {
            self.owner_type(&binding);
            self.specializer.constraint_variables.insert(crate::span_key(variable.span), binding);
        }

        visit::walk_variable(self, variable)
    }
}
