//! Instantiate user type and function templates before checking concrete declarations.

mod canonical;
mod constraint;
pub(crate) mod constructor;
mod function;
mod inference;
mod lifecycle;
mod method;
mod substitute;
mod symbolic;

use crate::checker::SpanKey;
use slang_module::SymbolOrigin;
use slang_source::{Diagnostic, Span};
use slang_syntax::{
    visit::{self, VisitorMut},
    *,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone)]
struct Template {
    name: String,
    function: bool,
    parameters: Vec<GenericParameter>,
    declarations: Vec<Declaration>,
    nested: Vec<NestedTypeInfo>,
    statics: Vec<StaticMemberInfo>,
    owner: Option<NestedTypeInfo>,
}

struct Specializer {
    shapes: crate::checker::Checker,
    templates: BTreeMap<String, Template>,
    function_templates: BTreeMap<String, Vec<String>>,
    function_applications: BTreeMap<(String, Vec<String>), String>,
    ordinary_functions: BTreeSet<String>,
    aliases: BTreeMap<String, TypeRef>,
    nested: BTreeMap<(String, String), String>,
    symbols: BTreeMap<String, SymbolOrigin>,
    public_types: BTreeSet<String>,
    instances: BTreeMap<(String, Vec<String>), String>,
    pending: VecDeque<Declaration>,
    nested_output: Vec<NestedTypeInfo>,
    static_output: Vec<StaticMemberInfo>,
    instance_output: Vec<GenericInstanceInfo>,
    application_output: Vec<FunctionApplicationInfo>,
    prefix: String,
    next_expression: usize,
    next_context: usize,
    remaining_nodes: usize,
    depth: usize,
    symbolic_types: BTreeSet<String>,
    inference_calls: BTreeMap<ExprId, InferenceCall>,
    generated: BTreeSet<SpanKey>,
    active: BTreeSet<SpanKey>,
    validation_contexts: BTreeSet<usize>,
    method_templates: BTreeMap<(String, String), String>,
    method_names: BTreeSet<String>,
    constructor_templates: BTreeMap<String, GenericConstructorInfo>,
    constructor_applications: BTreeMap<constructor::ConstructorKey, String>,
}

struct InferenceCall {
    name: String,
    supplied: Vec<TypeArgument>,
    explicit: bool,
    span: Span,
    module: slang_module::ModuleId,
}

#[derive(Default)]
pub(crate) struct InferenceProbe {
    pub pending: BTreeSet<ExprId>,
    pub types: BTreeMap<ExprId, Vec<crate::Type>>,
    pub shapes: BTreeMap<String, (crate::Type, Vec<u8>)>,
    pub deferred: BTreeSet<SpanKey>,
    pub function_names: BTreeMap<SpanKey, String>,
    pub selected_functions: BTreeSet<String>,
    pub ambiguities: Vec<Diagnostic>,
    pub methods: BTreeMap<(String, String), String>,
    pub targets: BTreeMap<ExprId, String>,
    pub constructor_requests:
        BTreeMap<constructor::ConstructorKey, constructor::ConstructorRequest>,
}

struct SpecializedModule {
    module: Module,
    symbols: BTreeMap<String, SymbolOrigin>,
    symbolic_types: BTreeSet<String>,
}

pub(super) fn specialize(
    module: Module,
    symbols: BTreeMap<String, SymbolOrigin>,
    major: slang_layout::MatrixMajor,
) -> Result<(Module, BTreeMap<String, SymbolOrigin>), Vec<Diagnostic>> {
    specialize_inner(module, symbols, false, major)
        .map(|result| (result.module, result.symbols))
        .map_err(|d| vec![d])
}

pub(super) fn validate_templates(
    module: &Module,
    symbols: &BTreeMap<String, SymbolOrigin>,
    major: slang_layout::MatrixMajor,
) -> Result<(), Vec<Diagnostic>> {
    if !module
        .declarations
        .iter()
        .any(|d| !parameters(d).is_empty())
    {
        return Ok(());
    }
    let result =
        specialize_inner(module.clone(), symbols.clone(), true, major).map_err(|d| vec![d])?;
    crate::checker::check_concrete(
        result.module,
        vec![],
        result.symbols,
        major,
        result.symbolic_types,
    )
    .map(|_| ())
}

fn specialize_inner(
    mut module: Module,
    symbols: BTreeMap<String, SymbolOrigin>,
    symbolic: bool,
    major: slang_layout::MatrixMajor,
) -> Result<SpecializedModule, Diagnostic> {
    if !module
        .declarations
        .iter()
        .any(|d| !parameters(d).is_empty())
    {
        return Ok(SpecializedModule {
            module,
            symbols,
            symbolic_types: BTreeSet::new(),
        });
    }
    let mut ids = Ids::default();
    for declaration in &mut module.declarations {
        ids.declaration(declaration)?;
    }
    let mut prefix = "_slang_generic_".to_owned();
    while module
        .declarations
        .iter()
        .any(|d| declaration_name(d).starts_with(&prefix))
        || symbols.keys().any(|n| n.starts_with(&prefix))
    {
        prefix.push('_');
    }
    let known: BTreeSet<_> = module
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Struct(s) => Some(s.name.clone()),
            Declaration::Interface(i) => Some(i.name.clone()),
            Declaration::TypeAlias(a) => Some(a.name.clone()),
            Declaration::Function(f) if !f.generic_parameters.is_empty() => Some(f.name.clone()),
            _ => None,
        })
        .chain(
            module
                .declarations
                .iter()
                .flat_map(|d| parameters(d).iter().map(|p| p.name.clone())),
        )
        .collect();
    let mut shapes = crate::checker::Checker::new(&module, symbols.clone(), major);
    for declaration in &module.declarations {
        match declaration {
            Declaration::Struct(s) => {
                shapes.structures.insert(s.name.clone(), BTreeMap::new());
                shapes.struct_definitions.insert(s.name.clone(), s.clone());
            }
            Declaration::Interface(i) => {
                shapes
                    .interface_definitions
                    .insert(i.name.clone(), i.clone());
            }
            Declaration::TypeAlias(a) if parameters(declaration).is_empty() => {
                shapes.aliases.insert(a.name.clone(), a.target.clone());
            }
            _ => {}
        }
    }
    let mut specializer = Specializer {
        shapes,
        public_types: module
            .declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Struct(s) if s.modifiers.iter().any(|m| m == "public") => {
                    Some(s.name.clone())
                }
                Declaration::Interface(i) if i.modifiers.iter().any(|m| m == "public") => {
                    Some(i.name.clone())
                }
                Declaration::TypeAlias(a) if a.modifiers.iter().any(|m| m == "public") => {
                    Some(a.name.clone())
                }
                _ => None,
            })
            .collect(),
        templates: BTreeMap::new(),
        function_templates: BTreeMap::new(),
        function_applications: BTreeMap::new(),
        ordinary_functions: BTreeSet::new(),
        aliases: BTreeMap::new(),
        nested: BTreeMap::new(),
        symbols,
        instances: BTreeMap::new(),
        pending: VecDeque::new(),
        nested_output: vec![],
        static_output: vec![],
        instance_output: vec![],
        application_output: vec![],
        prefix,
        next_expression: ids.expression,
        next_context: ids.context.max(1),
        remaining_nodes: 65536,
        depth: 0,
        symbolic_types: BTreeSet::new(),
        inference_calls: BTreeMap::new(),
        generated: BTreeSet::new(),
        active: BTreeSet::new(),
        validation_contexts: BTreeSet::new(),
        method_templates: BTreeMap::new(),
        method_names: BTreeSet::new(),
        constructor_templates: BTreeMap::new(),
        constructor_applications: BTreeMap::new(),
    };
    let declarations = std::mem::take(&mut module.declarations);
    specializer.register(
        declarations,
        std::mem::take(&mut module.nested_types),
        std::mem::take(&mut module.static_members),
        Some(&known),
    )?;
    let mut validated = BTreeSet::new();
    loop {
        if symbolic
            && let Some(name) = specializer
                .templates
                .keys()
                .find(|n| !validated.contains(*n) && specializer.validation_template(n))
                .cloned()
        {
            validated.insert(name.clone());
            specializer.symbolic_instance(&name)?;
            continue;
        }
        let Some(mut declaration) = specializer.pending.pop_front() else {
            break;
        };
        specializer.declaration(&mut declaration)?;
        module.declarations.push(declaration);
    }
    specializer.infer_calls(&mut module, major)?;
    Ok(SpecializedModule {
        module,
        symbols: specializer.symbols,
        symbolic_types: specializer.symbolic_types,
    })
}

impl Specializer {
    fn register(
        &mut self,
        declarations: Vec<Declaration>,
        nested: Vec<NestedTypeInfo>,
        statics: Vec<StaticMemberInfo>,
        known: Option<&BTreeSet<String>>,
    ) -> Result<(), Diagnostic> {
        for declaration in &declarations {
            if let Declaration::Function(function) = declaration
                && !function.generic_parameters.is_empty()
                && let Some(method) = &function.method
            {
                self.method_names.insert(method.name.clone());
            }
        }
        let owners: BTreeMap<_, _> = nested
            .iter()
            .map(|n| (n.type_name.clone(), n.owner.clone()))
            .collect();
        let static_owners: BTreeMap<_, _> = statics
            .iter()
            .map(|s| (s.variable_name.clone(), s.owner.clone()))
            .collect();
        let roots: BTreeSet<_> = declarations
            .iter()
            .filter(|d| !matches!(d, Declaration::Function(_)) && !parameters(d).is_empty())
            .map(|d| declaration_name(d).to_owned())
            .collect();
        let containing = |name: &str| {
            let mut current = Some(name);
            let mut found = None;
            while let Some(name) = current {
                if roots.contains(name) {
                    found = Some(name.to_owned());
                }
                current = owners.get(name).map(String::as_str);
            }
            found
        };
        let root_for = |declaration: &Declaration| {
            let name = match declaration {
                Declaration::Function(f) => f.owner_type().unwrap_or(&f.name),
                Declaration::Global(v) => static_owners
                    .get(&v.name)
                    .map_or(v.name.as_str(), String::as_str),
                _ => declaration_name(declaration),
            };
            containing(name)
        };
        let mut templates = BTreeMap::<String, Template>::new();
        for declaration in &declarations {
            if !parameters(declaration).is_empty()
                && containing(declaration_name(declaration)).as_deref()
                    == Some(declaration_name(declaration))
            {
                let name = declaration_name(declaration).to_owned();
                constraint::validate_parameters(parameters(declaration))?;
                let mut names = BTreeSet::new();
                for parameter in parameters(declaration) {
                    if !names.insert(&parameter.name) {
                        return Err(Diagnostic::error(
                            "T003",
                            parameter.span,
                            "duplicate generic parameter",
                        ));
                    }
                }
                if templates.contains_key(&name) || self.templates.contains_key(&name) {
                    return Err(error(
                        declaration_span(declaration),
                        "duplicate generic type declaration",
                    ));
                }
                templates.insert(
                    name.clone(),
                    Template {
                        name,
                        function: false,
                        parameters: parameters(declaration).to_vec(),
                        declarations: vec![],
                        nested: vec![],
                        statics: vec![],
                        owner: None,
                    },
                );
            }
        }
        let mut functions = vec![];
        for mut declaration in declarations {
            if match &declaration {
                Declaration::Struct(s) => s.modifiers.iter().any(|m| m == "public"),
                Declaration::Interface(i) => i.modifiers.iter().any(|m| m == "public"),
                Declaration::TypeAlias(a) => a.modifiers.iter().any(|m| m == "public"),
                _ => false,
            } {
                self.public_types
                    .insert(declaration_name(&declaration).into());
            }
            if let Some(root) = root_for(&declaration) {
                if let Some(known) = known {
                    let module = self.symbols.get(&root).map_or(0, |o| o.module.0);
                    Validator {
                        known,
                        symbols: &self.symbols,
                        module,
                        method_names: &self.method_names,
                    }
                    .declaration(&mut declaration)?;
                }
                templates
                    .get_mut(&root)
                    .unwrap()
                    .declarations
                    .push(declaration);
            } else {
                if matches!(&declaration, Declaration::Function(f) if !f.generic_parameters.is_empty())
                {
                    functions.push(declaration);
                    continue;
                }
                if let Declaration::Function(f) = &declaration {
                    self.ordinary_functions.insert(f.name.clone());
                }
                if let Declaration::TypeAlias(alias) = &declaration {
                    self.aliases
                        .insert(alias.name.clone(), alias.target.clone());
                }
                if let Declaration::Struct(s) = &declaration {
                    self.shapes
                        .struct_definitions
                        .insert(s.name.clone(), s.clone());
                    self.shapes
                        .structures
                        .insert(s.name.clone(), BTreeMap::new());
                }
                if let Declaration::Interface(i) = &declaration {
                    self.shapes
                        .interface_definitions
                        .insert(i.name.clone(), i.clone());
                }
                if let Declaration::Global(v) = &declaration {
                    self.shapes.shape_globals.insert(v.name.clone(), v.clone());
                }
                self.pending.push_back(declaration);
            }
        }
        for member in nested {
            if let Some(root) = containing(&member.type_name) {
                if root == member.type_name {
                    self.nested.insert(
                        (member.owner.clone(), member.name.clone()),
                        member.type_name.clone(),
                    );
                    templates.get_mut(&root).unwrap().owner = Some(member);
                } else {
                    templates.get_mut(&root).unwrap().nested.push(member);
                }
            } else {
                self.nested.insert(
                    (member.owner.clone(), member.name.clone()),
                    member.type_name.clone(),
                );
                self.nested_output.push(member);
            }
        }
        for member in statics {
            if let Some(root) = containing(&member.owner) {
                templates.get_mut(&root).unwrap().statics.push(member);
            } else {
                self.shapes.static_fields.insert(
                    (member.owner.clone(), member.name.clone()),
                    member.variable_name.clone(),
                );
                self.shapes
                    .static_owners
                    .insert(member.variable_name.clone(), member.owner.clone());
                self.static_output.push(member);
            }
        }
        self.templates.extend(templates);
        self.register_functions(functions, known)?;
        Ok(())
    }

    fn instantiate(
        &mut self,
        name: &str,
        supplied: &[TypeArgument],
        span: Span,
    ) -> Result<String, Diagnostic> {
        if self.depth >= 32 {
            return Err(error(span, "generic specialization limit exceeded"));
        }
        let template = self.templates[name].clone();
        if supplied.len() > template.parameters.len() {
            return Err(error(span, "too many generic type arguments"));
        }
        self.depth += 1;
        let result = self.instantiate_inner(name, template, supplied, span);
        self.depth -= 1;
        result
    }

    fn instantiate_inner(
        &mut self,
        template_key: &str,
        template: Template,
        supplied: &[TypeArgument],
        span: Span,
    ) -> Result<String, Diagnostic> {
        let mut substitutions = BTreeMap::new();
        let mut arguments = vec![];
        for (index, parameter) in template.parameters.iter().enumerate() {
            let mut argument = match supplied.get(index) {
                Some(TypeArgument::Type(ty)) => ty.clone(),
                Some(_) => {
                    return Err(error(
                        span,
                        "generic type parameter requires a type argument",
                    ));
                }
                None => parameter
                    .default
                    .clone()
                    .ok_or_else(|| error(span, "missing generic type argument"))?,
            };
            substitute::replace_type(&mut argument, &substitutions);
            self.canonical(&mut argument, 0)?;
            self.validate_argument(&mut argument)?;
            substitutions.insert(parameter.name.clone(), argument.clone());
            arguments.push(argument);
        }
        if !self.constraints_satisfied(&template.parameters, &substitutions)? {
            return Err(Diagnostic::error(
                "T033",
                span,
                "generic type arguments do not satisfy the interface constraints",
            ));
        }
        let key = (
            template_key.to_owned(),
            arguments
                .iter()
                .map(canonical::type_key)
                .collect::<Vec<_>>(),
        );
        if let Some(name) = self.instances.get(&key) {
            return Ok(name.clone());
        }
        if self.instances.len() >= 1024 {
            return Err(error(span, "generic specialization limit exceeded"));
        }
        let context = self.next_context;
        self.next_context += 1;
        let target = format!("{}{}", self.prefix, context);
        self.instances.insert(key, target.clone());
        let names: BTreeMap<_, _> = template
            .declarations
            .iter()
            .enumerate()
            .map(|(index, d)| {
                (
                    declaration_name(d).to_owned(),
                    if declaration_name(d) == template.name {
                        target.clone()
                    } else {
                        format!("{target}_member_{index}")
                    },
                )
            })
            .collect();
        let parent_context = template
            .declarations
            .first()
            .map_or(0, |d| declaration_span(d).context);
        let mut declarations = template.declarations;
        let mut visitor = substitute::Substitute {
            root: &template.name,
            names: &names,
            types: &substitutions,
            context,
            next_expression: &mut self.next_expression,
            remaining_nodes: &mut self.remaining_nodes,
        };
        for declaration in &mut declarations {
            if declaration_name(declaration) == template.name {
                clear_parameters(declaration);
            }
            visitor.declaration(declaration)?;
        }
        self.generated.extend(
            declarations
                .iter()
                .map(|d| crate::span_key(declaration_span(d))),
        );
        let source = self
            .symbols
            .get(&template.name)
            .map_or(template.name.clone(), |o| o.name.clone());
        let display = format!(
            "{source}<{}>",
            arguments
                .iter()
                .map(canonical::type_key)
                .collect::<Vec<_>>()
                .join(", ")
        );
        for (old, new) in &names {
            let mut origin = self
                .symbols
                .get(old)
                .cloned()
                .unwrap_or_else(|| SymbolOrigin {
                    module: Default::default(),
                    name: old.clone(),
                });
            origin.name = if old == &template.name {
                display.clone()
            } else {
                origin.name.replacen(&source, &display, 1)
            };
            self.symbols.insert(new.clone(), origin);
        }
        self.shapes.symbols = self.symbols.clone();
        let nested = template
            .nested
            .into_iter()
            .map(|mut n| {
                n.owner = names[&n.owner].clone();
                n.type_name = names[&n.type_name].clone();
                n.span = n.span.with_context(context);
                n
            })
            .collect();
        let statics = template
            .statics
            .into_iter()
            .map(|mut s| {
                s.owner = names[&s.owner].clone();
                s.variable_name = names[&s.variable_name].clone();
                s.span = s.span.with_context(context);
                s
            })
            .collect();
        if let Some(parent) = self
            .instance_output
            .iter()
            .find(|i| i.context == parent_context)
        {
            arguments.extend(parent.arguments.clone());
        }
        self.instance_output.push(GenericInstanceInfo {
            context,
            type_name: target.clone(),
            template_name: template.name.clone(),
            parameter_count: template.parameters.len(),
            arguments,
        });
        if let Some(mut owner) = template.owner {
            owner.type_name = target.clone();
            // Keep lexical ownership without conflating distinct applications in member lookup.
            owner.name = target.clone();
            owner.span = owner.span.with_context(context);
            self.nested_output.push(owner);
        }
        self.register(declarations, nested, statics, None)?;
        Ok(target)
    }

    fn expand(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if self.function_templates.contains_key(&ty.name) {
            return Err(error(ty.span, "generic function is not a type"));
        }
        if ty.base.is_none() && ty.name.contains("::") {
            let mut parts = ty.name.split("::");
            let mut name = parts.next().unwrap().to_owned();
            for member in parts {
                if let Some(target) = self.nested.get(&(name.clone(), member.into())) {
                    self.accessible(target, ty.span)?;
                    name = target.clone();
                } else {
                    name.push_str("::");
                    name.push_str(member);
                }
            }
            ty.name = name;
        }
        if let Some(mut base) = ty.base.take() {
            self.expand(&mut base)?;
            self.canonical(&mut base, 0)?;
            ty.name = self
                .nested
                .get(&(base.name.clone(), ty.name.clone()))
                .cloned()
                .ok_or_else(|| error(ty.span, "unknown nested generic type"))?;
            self.accessible(&ty.name, ty.span)?;
        }
        if self.templates.contains_key(&ty.name) {
            if self.templates[&ty.name].function {
                return Err(error(ty.span, "generic function is not a type"));
            }
            if !ty.application {
                return Err(error(
                    ty.span,
                    "generic type requires explicit angle brackets",
                ));
            }
            ty.name = self.instantiate(&ty.name, &ty.arguments, ty.span)?;
            ty.arguments.clear();
            ty.application = false;
        } else {
            for argument in &mut ty.arguments {
                if let TypeArgument::Type(argument) = argument {
                    self.expand(argument)?;
                }
            }
        }
        Ok(())
    }

    fn accessible(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if self
            .symbols
            .get(name)
            .is_some_and(|o| o.module != self.shapes.current_module)
            && !self.public_types.contains(name)
        {
            return Err(Diagnostic::error(
                "T018",
                span,
                "nested type is internal to its module",
            ));
        }
        Ok(())
    }

    fn validate_argument(&self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if !canonical::core_type(&ty.name)
            && !self.shapes.structures.contains_key(&ty.name)
            && !self.shapes.interface_definitions.contains_key(&ty.name)
            && !self.aliases.contains_key(&ty.name)
        {
            return Err(error(
                ty.span,
                format!("unknown generic argument type '{}'", ty.name),
            ));
        }
        let known = self
            .shapes
            .structures
            .keys()
            .chain(self.shapes.interface_definitions.keys())
            .chain(self.aliases.keys())
            .cloned()
            .collect();
        Validator {
            known: &known,
            symbols: &self.symbols,
            module: 0,
            method_names: &self.method_names,
        }
        .ty(ty)
    }
}

impl VisitorMut for Specializer {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        self.expand(ty)?;
        visit::walk_type(self, ty)
    }
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        self.normalize_member_application(expression)?;
        if let ExprKind::Call { callee, arguments } = &mut expression.kind {
            self.normalize_member_application(callee)?;
            let application = match &callee.kind {
                ExprKind::Name(name)
                    if self.function_templates.contains_key(name)
                        || self.shapes.current_static_type.is_some()
                            && self.method_names.contains(name) =>
                {
                    Some((name.clone(), vec![], false))
                }
                ExprKind::Member { name, .. } if self.method_names.contains(name) => {
                    Some((name.clone(), vec![], false))
                }
                ExprKind::GenericApplication { callee, arguments } => match &callee.kind {
                    ExprKind::Member { name, .. } => Some((name.clone(), arguments.clone(), true)),
                    _ => None,
                },
                ExprKind::TypeApplication(reference)
                    if reference.base.is_none()
                        && (self.function_templates.contains_key(&reference.name)
                            || self.shapes.current_static_type.is_some()
                                && self.method_names.contains(&reference.name)) =>
                {
                    if !reference.arrays.is_empty() || reference.matrix_major.is_some() {
                        return Err(error(
                            reference.span,
                            "invalid generic function application",
                        ));
                    }
                    Some((reference.name.clone(), reference.arguments.clone(), true))
                }
                _ => None,
            };
            if let Some((name, mut supplied, explicit)) = application {
                for argument in &mut supplied {
                    let TypeArgument::Type(argument) = argument else {
                        return Err(error(
                            callee.span,
                            "generic type parameter requires a type argument",
                        ));
                    };
                    self.canonical(argument, 0)?;
                    self.validate_argument(argument)?;
                }
                self.inference_calls.insert(
                    callee.id,
                    InferenceCall {
                        name,
                        supplied,
                        explicit,
                        span: callee.span,
                        module: self.shapes.current_module,
                    },
                );
                self.member_receiver(callee)?;
                for argument in arguments {
                    self.expression(argument)?;
                }
                return Ok(());
            }
        }
        if let ExprKind::TypeApplication(reference) = &expression.kind
            && reference.base.is_none()
            && self.function_templates.contains_key(&reference.name)
        {
            if !reference.arrays.is_empty() || reference.matrix_major.is_some() {
                return Err(error(
                    reference.span,
                    "invalid generic function application",
                ));
            }
            let name =
                self.apply_function(&reference.name, &reference.arguments, expression.span)?;
            expression.kind = ExprKind::Name(name);
            return Ok(());
        }
        if let ExprKind::Name(name) = &expression.kind
            && (self.templates.contains_key(name) || self.function_templates.contains_key(name))
            && !self.ordinary_functions.contains(name)
        {
            return Err(error(
                expression.span,
                if self.function_templates.contains_key(name) {
                    "generic function requires explicit type arguments"
                } else {
                    "generic constructor requires explicit type arguments"
                },
            ));
        }
        visit::walk_expression(self, expression)
    }
    fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        self.shapes.current_static_type = match declaration {
            Declaration::Function(function) => function.owner_type().map(str::to_owned),
            Declaration::Struct(structure) => Some(structure.name.clone()),
            Declaration::Global(variable) => self
                .static_output
                .iter()
                .find(|s| s.variable_name == variable.name)
                .map(|s| s.owner.clone()),
            _ => None,
        };
        self.shapes.current_module = self
            .symbols
            .get(declaration_name(declaration))
            .map_or(Default::default(), |o| o.module);
        let site = crate::span_key(declaration_span(declaration));
        if self.generated.contains(&site)
            && !self.active.contains(&site)
            && !self
                .validation_contexts
                .contains(&declaration_span(declaration).context)
        {
            return self.signature_declaration(declaration);
        }
        self.active.insert(site);
        if matches!(declaration, Declaration::Function(_)) {
            self.shapes.scopes.push(BTreeMap::new());
        }
        visit::walk_declaration(self, declaration)?;
        if matches!(declaration, Declaration::Function(_)) {
            self.shapes.scopes.pop();
        }
        Ok(())
    }
    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        visit::walk_variable(self, variable)?;
        if self.shapes.scopes.len() > 1 {
            let ty = crate::Type::builtin(&variable.ty.name).unwrap_or(crate::Type::Error);
            let constant = if variable.modifiers.iter().any(|m| m == "const") {
                variable
                    .initializer
                    .as_ref()
                    .and_then(|e| self.shape_value(e))
            } else {
                None
            };
            self.shapes.scopes.last_mut().unwrap().insert(
                variable.name.clone(),
                crate::checker::Binding {
                    ty,
                    writable: false,
                    constant,
                    shape_constant: constant,
                    storage: crate::checker::Storage::Local,
                },
            );
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        let scope = matches!(
            statement.kind,
            StmtKind::Block(_) | StmtKind::For { .. } | StmtKind::Switch { .. }
        );
        if scope {
            self.shapes.scopes.push(BTreeMap::new());
        }
        visit::walk_statement(self, statement)?;
        if scope {
            self.shapes.scopes.pop();
        }
        Ok(())
    }
}

#[derive(Default)]
struct Ids {
    expression: usize,
    context: usize,
}
impl VisitorMut for Ids {
    fn span(&mut self, span: &mut Span) {
        self.context = self.context.max(span.context + 1);
    }
    fn expression_id(&mut self, id: &mut ExprId) {
        self.expression = self.expression.max(id.0 + 1);
    }
}

struct Validator<'a> {
    known: &'a BTreeSet<String>,
    symbols: &'a BTreeMap<String, SymbolOrigin>,
    module: usize,
    method_names: &'a BTreeSet<String>,
}
impl VisitorMut for Validator<'_> {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::Call { callee, arguments } = &mut expression.kind
            && let ExprKind::TypeApplication(reference) = &mut callee.kind
            && reference.base.is_none()
            && self.method_names.contains(&reference.name)
        {
            visit::walk_type(self, reference)?;
            for argument in arguments {
                self.expression(argument)?;
            }
            return Ok(());
        }
        visit::walk_expression(self, expression)
    }

    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if ty.base.is_none()
            && !ty.name.contains("::")
            && !canonical::core_type(&ty.name)
            && (!self.known.contains(&ty.name)
                || self.module != 0 && self.symbols.get(&ty.name).is_some_and(|o| o.module.0 == 0))
        {
            return Err(error(
                ty.span,
                format!("unknown type '{}' in generic declaration", ty.name),
            ));
        }
        visit::walk_type(self, ty)
    }
}

pub(super) fn argument_names(arguments: &[TypeRef]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut pending = arguments.iter().collect::<Vec<_>>();
    while let Some(ty) = pending.pop() {
        names.insert(ty.name.clone());
        if let Some(base) = &ty.base {
            pending.push(base);
        }
        for argument in &ty.arguments {
            if let TypeArgument::Type(ty) = argument {
                pending.push(ty);
            }
        }
    }
    names
}

fn parameters(declaration: &Declaration) -> &[GenericParameter] {
    match declaration {
        Declaration::Struct(s) => &s.generic_parameters,
        Declaration::Interface(i) => &i.generic_parameters,
        Declaration::TypeAlias(a) => &a.generic_parameters,
        Declaration::Function(f) => &f.generic_parameters,
        _ => &[],
    }
}
fn clear_parameters(declaration: &mut Declaration) {
    match declaration {
        Declaration::Struct(s) => s.generic_parameters.clear(),
        Declaration::Interface(i) => i.generic_parameters.clear(),
        Declaration::TypeAlias(a) => a.generic_parameters.clear(),
        Declaration::Function(f) => f.generic_parameters.clear(),
        _ => {}
    }
}
fn declaration_name(declaration: &Declaration) -> &str {
    match declaration {
        Declaration::Struct(s) => &s.name,
        Declaration::Interface(i) => &i.name,
        Declaration::TypeAlias(a) => &a.name,
        Declaration::Global(v) => &v.name,
        Declaration::Function(f) => &f.name,
    }
}
fn declaration_span(declaration: &Declaration) -> Span {
    match declaration {
        Declaration::Struct(s) => s.span,
        Declaration::Interface(i) => i.span,
        Declaration::TypeAlias(a) => a.span,
        Declaration::Global(v) => v.span,
        Declaration::Function(f) => f.span,
    }
}
fn error(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error("T002", span, message)
}
