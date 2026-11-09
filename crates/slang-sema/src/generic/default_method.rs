//! Check interface bodies on opaque self types and instantiate missing witnesses.

use super::*;
use crate::{Type, checker::Checker};

pub(super) struct Context {
    pub(super) owner: String,
    interface: String,
    is_static: bool,
    members: BTreeSet<String>,
}

pub(super) fn has_bodies(declaration: &Declaration) -> bool {
    matches!(declaration, Declaration::Interface(i) if i.requirements.iter().any(|f| f.body.is_some()))
}

impl Specializer {
    pub(super) fn capture_default_selections(&mut self, probe: &InferenceProbe) -> bool {
        if !self.symbolic {
            return false;
        }
        let mut changed = false;
        for context in self.default_method_contexts.values() {
            if !self.symbolic_types.contains(&context.owner) {
                continue;
            }
            changed |= self
                .checked_default_interfaces
                .insert(context.interface.clone());
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
            && !self
                .shapes
                .scopes
                .iter()
                .skip(1)
                .any(|s| s.contains_key(name))
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
        self.shapes.current_module = self
            .symbols
            .get(&interface.name)
            .map_or(Default::default(), |o| o.module);
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
        let module = self
            .symbols
            .get(&interface.name)
            .map_or(Default::default(), |o| o.module);
        self.symbolic_types.insert(owner.clone());
        self.symbolic_parameters
            .insert(owner.clone(), witness::reference("This", interface.span));
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
        let names = self.default_names(&owner, interface, &BTreeMap::new());
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

    fn default_names(
        &self,
        owner: &str,
        interface: &Interface,
        methods: &BTreeMap<(String, String), String>,
    ) -> BTreeMap<String, String> {
        let mut names = BTreeMap::from([(interface.name.clone(), owner.into())]);
        for requirement in &interface.requirements {
            if let Some(method) = &requirement.method {
                let target = methods
                    .get(&(owner.into(), method.name.clone()))
                    .cloned()
                    .unwrap_or_else(|| format!("{}default_{owner}_{}", self.prefix, method.name));
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
        let name = declaration_name(&declaration).to_owned();
        if let Declaration::Function(function) = &mut declaration {
            function.modifiers.push("override".into());
        }
        let module = self
            .symbols
            .get(&interface.name)
            .map_or(Default::default(), |o| o.module);
        self.symbols.insert(
            name.clone(),
            SymbolOrigin {
                module,
                name: format!("{owner}.{}", requirement.method.as_ref().unwrap().name),
            },
        );
        self.generated
            .insert(crate::span_key(declaration_span(&declaration)));
        self.default_method_contexts.insert(
            context,
            Context {
                owner: owner.into(),
                interface: interface.name.clone(),
                is_static: requirement.method.as_ref().unwrap().is_static,
                members: self.default_members(interface),
            },
        );
        self.instance_output.push(GenericInstanceInfo {
            context,
            type_name: name,
            template_name: requirement.name.clone(),
            parameter_count: 0,
            arguments: vec![witness::reference(owner, requirement.span)],
        });
        Ok(declaration)
    }

    pub(super) fn synthesize_default_methods(
        &mut self,
        module: &Module,
        major: slang_layout::MatrixMajor,
    ) -> Result<bool, Diagnostic> {
        if !module.declarations.iter().any(has_bodies) {
            return Ok(false);
        }
        let mut checker = Checker::new(module, self.symbols.clone(), major);
        checker.symbolic_types = self.symbolic_types.clone();
        checker.declare(module);
        checker.declare_conformances(module);
        let mut candidates =
            BTreeMap::<(String, String, Vec<String>, bool), Vec<(String, Function)>>::new();
        for structure in checker
            .struct_definitions
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            if checker.symbolic_types.contains(&structure.name) {
                continue;
            }
            let interfaces = checker
                .conformances
                .get(&structure.name)
                .cloned()
                .unwrap_or_default();
            for name in interfaces {
                let interface = checker.interface_definitions[&name].clone();
                if self.symbolic && !self.checked_default_interfaces.contains(&name) {
                    continue;
                }
                for requirement in &interface.requirements {
                    if requirement.body.is_none()
                        || !requirement.generic_parameters.is_empty()
                        || requirement.method.is_none()
                        || checker
                            .direct_witnesses
                            .contains_key(&crate::conformance::witness::key(
                                &structure.name,
                                &name,
                                requirement.span,
                            ))
                    {
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
                    candidates
                        .entry(key)
                        .or_default()
                        .push((name.clone(), requirement.clone()));
                }
            }
        }
        let changed = !candidates.is_empty();
        for ((owner, method, _, _), requirements) in candidates {
            let selected = requirements
                .iter()
                .filter(|(name, _)| {
                    !requirements.iter().any(|(other, _)| {
                        other != name
                            && checker
                                .conformances
                                .get(other)
                                .is_some_and(|bases| bases.contains(name))
                    })
                })
                .collect::<Vec<_>>();
            if selected.len() != 1 {
                return Err(Diagnostic::error(
                    "T032",
                    checker.struct_definitions[&owner].span,
                    format!("ambiguous default interface method '{method}'"),
                ));
            }
            let (name, requirement) = selected[0];
            let interface = &checker.interface_definitions[name];
            let names = self.default_names(&owner, interface, &checker.methods);
            let declaration = self.clone_default(&owner, interface, requirement, &names, None)?;
            self.pending.push_back(declaration);
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
        self.types.insert(
            binding.owner.name.clone(),
            witness::reference(self.owner, binding.owner.span),
        );
    }
}

impl VisitorMut for SeedBindings<'_> {
    fn expression_id(&mut self, id: &mut ExprId) {
        let original = self
            .specializer
            .expression_origins
            .get(id)
            .copied()
            .unwrap_or(*id);
        if let Some(binding) = self
            .specializer
            .default_body_calls
            .get(&(self.interface.into(), original))
            .cloned()
        {
            self.owner_type(&binding);
            self.specializer.constraint_calls.insert(*id, binding);
        }
    }

    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        let span = crate::span_key(variable.span.with_context(0));
        if let Some(binding) = self
            .specializer
            .default_body_variables
            .get(&(self.interface.into(), span))
            .cloned()
        {
            self.owner_type(&binding);
            self.specializer
                .constraint_variables
                .insert(crate::span_key(variable.span), binding);
        }
        visit::walk_variable(self, variable)
    }
}
