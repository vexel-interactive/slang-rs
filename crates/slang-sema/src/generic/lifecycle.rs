//! Keep speculative declarations dormant until their types or calls are used.

use super::*;
use crate::span_key;

impl Specializer {
    pub(super) fn validation_template(&self, name: &str) -> bool {
        self.templates[name].declarations.first().is_some_and(|d| {
            !self.generated.contains(&span_key(declaration_span(d)))
                || self
                    .validation_contexts
                    .contains(&declaration_span(d).context)
        })
    }

    pub(super) fn signature_declaration(
        &mut self,
        declaration: &mut Declaration,
    ) -> Result<(), Diagnostic> {
        match declaration {
            Declaration::Function(function) => {
                self.ty(&mut function.return_type)?;
                for parameter in &mut function.parameters {
                    self.ty(&mut parameter.ty)?;
                }
            }
            Declaration::Struct(structure) => {
                for base in &mut structure.bases {
                    self.ty(base)?;
                }
                for field in &mut structure.fields {
                    self.ty(&mut field.ty)?;
                }
            }
            Declaration::Interface(interface) => {
                for base in &mut interface.bases {
                    self.ty(base)?;
                }
                for requirement in &mut interface.requirements {
                    self.ty(&mut requirement.return_type)?;
                    for parameter in &mut requirement.parameters {
                        self.ty(&mut parameter.ty)?;
                    }
                }
            }
            Declaration::TypeAlias(alias) => self.ty(&mut alias.target)?,
            Declaration::Global(variable) => self.ty(&mut variable.ty)?,
        }
        Ok(())
    }

    pub(super) fn drain_pending(&mut self, module: &mut Module) -> Result<bool, Diagnostic> {
        let mut changed = false;
        while let Some(mut declaration) = self.pending.pop_front() {
            self.declaration(&mut declaration)?;
            module.declarations.push(declaration);
            changed = true;
        }
        Ok(changed)
    }

    pub(super) fn activate_referenced(&mut self, module: &mut Module) -> Result<bool, Diagnostic> {
        let mut references = References::default();
        for declaration in &mut module.declarations {
            let site = span_key(declaration_span(declaration));
            if !self.generated.contains(&site) || self.active.contains(&site) {
                references.declaration(declaration)?;
            }
        }
        references.constraint_calls(&mut module.constraint_calls)?;
        loop {
            let mut changed = false;
            let before = references.0.len();
            for instance in &self.instance_output {
                if references.0.contains(&instance.type_name) {
                    for argument in &instance.arguments {
                        references.ty(&mut argument.clone())?;
                    }
                }
            }
            changed |= references.0.len() != before;
            for nested in &self.nested_output {
                if references.0.contains(&nested.type_name) {
                    changed |= references.0.insert(nested.owner.clone());
                }
            }
            if !changed {
                break;
            }
        }
        let contexts = module
            .declarations
            .iter()
            .filter_map(|d| {
                (!matches!(d, Declaration::Function(_))
                    && references.0.contains(declaration_name(d)))
                .then_some(declaration_span(d).context)
            })
            .collect::<BTreeSet<_>>();
        let mut changed = false;
        for declaration in &mut module.declarations {
            let span = declaration_span(declaration);
            let site = span_key(span);
            if !matches!(declaration, Declaration::Function(_))
                && contexts.contains(&span.context)
                && self.generated.contains(&site)
                && self.active.insert(site)
            {
                self.declaration(declaration)?;
                changed = true;
            }
        }
        Ok(changed)
    }

    pub(super) fn activate_selected(
        &mut self,
        module: &mut Module,
        probe: &InferenceProbe,
    ) -> Result<bool, Diagnostic> {
        let mut changed = false;
        for declaration in &mut module.declarations {
            if let Declaration::Function(function) = declaration {
                let site = span_key(function.span);
                if self.generated.contains(&site)
                    && !self.active.contains(&site)
                    && probe
                        .function_names
                        .get(&site)
                        .is_some_and(|n| probe.selected_functions.contains(n))
                {
                    self.active.insert(site);
                    self.declaration(declaration)?;
                    changed = true;
                }
            }
        }
        Ok(changed)
    }

    pub(super) fn active_diagnostic(&self, module: &Module, diagnostic: &Diagnostic) -> bool {
        !module.declarations.iter().any(|d| {
            let span = declaration_span(d);
            self.generated.contains(&span_key(span))
                && !self.active.contains(&span_key(span))
                && span.source == diagnostic.span.source
                && span.context == diagnostic.span.context
                && span.start <= diagnostic.span.start
                && diagnostic.span.end <= span.end
        })
    }

    pub(super) fn prune_dormant(&self, module: &mut Module) {
        module.declarations.retain(|d| {
            let site = span_key(declaration_span(d));
            !self.generated.contains(&site) || self.active.contains(&site)
        });
        let mut references = References::default();
        for declaration in &mut module.declarations {
            references
                .declaration(declaration)
                .expect("reference traversal cannot fail");
        }
        references
            .constraint_calls(&mut module.constraint_calls)
            .expect("reference traversal cannot fail");
        module
            .constraint_calls
            .retain(|id, _| references.1.contains(id));
        let names = module
            .declarations
            .iter()
            .map(|d| declaration_name(d).to_owned())
            .collect::<BTreeSet<_>>();
        module
            .nested_types
            .retain(|n| names.contains(&n.owner) && names.contains(&n.type_name));
        module
            .static_members
            .retain(|s| names.contains(&s.owner) && names.contains(&s.variable_name));
        module
            .generic_instances
            .retain(|i| names.contains(&i.type_name));
        for application in &mut module.function_applications {
            application
                .candidates
                .retain(|c| names.contains(&c.function));
        }
        module.function_applications.retain(|a| {
            !a.candidates.is_empty() || self.constructor_applications.values().any(|n| n == &a.name)
        });
        module
            .generic_constructors
            .retain(|c| names.contains(&c.owner));
        module
            .constructor_applications
            .retain(|c| names.contains(&c.owner));
    }
}

impl InferenceProbe {
    pub(crate) fn record_selections(&mut self, checker: &crate::checker::Checker) {
        self.function_names = checker.function_names.clone();
        self.targets = checker.generic_call_names.clone();
        self.constructor_requests = checker.generic_constructor_requests.borrow().clone();
        self.selected_functions = checker.call_names.values().cloned().collect();
        for ((owner, ..), witness) in &checker.direct_witnesses {
            if checker
                .struct_definitions
                .get(owner)
                .is_some_and(|s| !self.deferred.contains(&span_key(s.span)))
                && let Some(name) = checker.function_names.get(&witness.declaration)
            {
                self.selected_functions.insert(name.clone());
            }
        }
        self.ambiguities = checker
            .diagnostics
            .iter()
            .filter(|d| d.code == "T014" && d.message.contains("ambiguous"))
            .cloned()
            .collect();
        let mut plans = checker
            .initializations
            .values()
            .chain(checker.variable_initializations.values())
            .collect::<Vec<_>>();
        plans.extend(checker.default_arguments.values().flatten());
        plans.extend(
            checker
                .constructor_initializations
                .values()
                .flatten()
                .map(|(_, p)| p),
        );
        while let Some(plan) = plans.pop() {
            match &plan.kind {
                crate::InitializationKind::ConstructorCall {
                    name, arguments, ..
                } => {
                    self.selected_functions.insert(name.clone());
                    plans.extend(arguments);
                }
                crate::InitializationKind::Aggregate(values) => plans.extend(values),
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct References(BTreeSet<String>, BTreeSet<ExprId>);
impl References {
    fn constraint_calls(
        &mut self,
        calls: &mut BTreeMap<ExprId, ConstraintCallInfo>,
    ) -> Result<(), Diagnostic> {
        let mut visited = BTreeSet::new();
        loop {
            let pending = calls
                .keys()
                .filter(|id| self.1.contains(*id) && !visited.contains(*id))
                .copied()
                .collect::<Vec<_>>();
            if pending.is_empty() {
                break;
            }
            for id in pending {
                visited.insert(id);
                let binding = calls.get_mut(&id).unwrap();
                self.ty(&mut binding.owner)?;
                self.ty(&mut binding.interface)?;
                visit::walk_function(self, &mut binding.requirement)?;
            }
        }
        Ok(())
    }
}
impl VisitorMut for References {
    fn expression_id(&mut self, id: &mut ExprId) {
        self.1.insert(*id);
    }
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        self.0.insert(ty.name.clone());
        visit::walk_type(self, ty)
    }
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::Name(name) = &expression.kind {
            self.0.insert(name.clone());
        }
        visit::walk_expression(self, expression)
    }
    fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        if let Declaration::Function(function) = declaration
            && let Some(owner) = function.owner_type()
        {
            self.0.insert(owner.to_owned());
        }
        visit::walk_declaration(self, declaration)
    }
}
