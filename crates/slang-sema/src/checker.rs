use crate::Type;
use serde::Serialize;
use slang_module::{ModuleId, ModuleInfo, Program, SymbolOrigin};
use slang_source::{Diagnostic, Severity, Span};
use slang_syntax::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct EntryPoint {
    pub name: String,
    pub stage: ShaderStage,
    /// Present only for compute shaders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_group_size: Option<[u32; 3]>,
}

/// Shader execution models currently checked by the native frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShaderStage {
    Compute,
    Vertex,
    Fragment,
}

#[derive(Clone, Debug)]
pub struct CheckedModule {
    pub matrix_major: slang_layout::MatrixMajor,
    pub module: Module,
    pub expression_types: BTreeMap<ExprId, Type>,
    /// Resource expressions implicitly loaded in their checked value context.
    pub constant_buffer_loads: BTreeSet<ExprId>,
    /// Constant scalar indexes into numeric member selections.
    pub swizzle_indices: BTreeMap<ExprId, u8>,
    /// Receiver types for unqualified field references in constructors and methods.
    pub implicit_members: BTreeMap<ExprId, Type>,
    pub method_receivers: BTreeMap<ExprId, crate::method::MethodReceiver>,
    /// Static field accesses, whose lookup base is never evaluated.
    pub static_references: BTreeMap<ExprId, String>,
    /// Nested type selections, whose value bases are checked but never evaluated.
    pub static_type_lookups: BTreeSet<ExprId>,
    pub variable_initializations: BTreeMap<SpanKey, crate::Initialization>,
    pub constructor_initializations: BTreeMap<SpanKey, Vec<(String, crate::Initialization)>>,
    pub declaration_types: BTreeMap<SpanKey, Type>,
    pub matrix_layouts: BTreeMap<SpanKey, slang_layout::MatrixMajor>,
    pub entry_points: Vec<EntryPoint>,
    pub modules: Vec<ModuleInfo>,
    pub symbols: BTreeMap<String, SymbolOrigin>,
    pub function_names: BTreeMap<SpanKey, String>,
    pub call_names: BTreeMap<ExprId, String>,
    /// Selected callee and writable argument positions, including resource methods.
    pub call_output_parameters: BTreeMap<SpanKey, (String, Vec<bool>)>,
    /// Resolved global variable declarations, independent of lexical spelling.
    pub global_references: BTreeMap<ExprId, SpanKey>,
    pub initializations: BTreeMap<ExprId, crate::Initialization>,
    pub default_arguments: BTreeMap<ExprId, Vec<crate::Initialization>>,
    /// Integer case values after conversion to their switch selector type.
    pub case_values: BTreeMap<ExprId, i64>,
    pub entry_point_requirements: BTreeMap<String, crate::EntryPointRequirements>,
    /// Checked scalar integer and boolean constants; uint64 values store bits.
    pub global_constants: BTreeMap<SpanKey, i64>,
    /// Allocated resource ranges, keyed by their source declarations.
    pub resource_bindings: BTreeMap<SpanKey, slang_layout::ResourceBinding>,
    /// Shared constant-buffer register for ordinary global uniform values.
    pub global_uniform_binding: Option<slang_layout::ResourceBinding>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Declaration location and instantiation context within a compilation.
pub type SpanKey = (usize, usize, usize, usize);
pub fn span_key(span: Span) -> SpanKey {
    (span.source.0, span.start, span.end, span.context)
}

#[derive(Clone)]
pub(crate) struct Binding {
    pub ty: Type,
    pub writable: bool,
    /// Integer values use signed storage; the binding type determines the sign.
    pub constant: Option<i64>,
    /// Template dimensions use upstream's wider integer intermediates.
    pub shape_constant: Option<i64>,
    pub storage: Storage,
}

#[derive(Clone, Copy)]
pub(crate) enum Storage {
    Local,
    Global(SpanKey),
    GroupShared(SpanKey),
}

#[derive(Clone)]
pub(crate) struct Signature {
    pub declaration: SpanKey,
    pub parameters: Vec<Type>,
    pub output_parameters: Vec<bool>,
    pub result: Type,
    pub defined: bool,
    pub public: bool,
    pub target_name: String,
    pub defaults: Vec<Option<Expr>>,
    pub method: Option<StructMethod>,
    pub default_type_arguments: usize,
    pub generic: bool,
}

pub(crate) struct Checker {
    pub default_matrix_major: slang_layout::MatrixMajor,
    pub scopes: Vec<BTreeMap<String, Binding>>,
    pub functions: BTreeMap<String, Vec<Signature>>,
    pub structures: BTreeMap<String, BTreeMap<String, Type>>,
    pub expression_types: BTreeMap<ExprId, Type>,
    pub constant_buffer_loads: BTreeSet<ExprId>,
    pub swizzle_indices: BTreeMap<ExprId, u8>,
    pub implicit_members: BTreeMap<ExprId, Type>,
    pub variable_initializations: BTreeMap<SpanKey, crate::Initialization>,
    pub constructor_initializations: BTreeMap<SpanKey, Vec<(String, crate::Initialization)>>,
    pub constructors: BTreeMap<String, String>,
    pub current_constructor: Option<String>,
    pub current_type: Option<String>,
    pub current_static_type: Option<String>,
    pub generic_type_contexts: BTreeMap<usize, BTreeSet<String>>,
    pub symbolic_types: BTreeSet<String>,
    pub symbolic_shapes: BTreeMap<String, (Type, Vec<u8>)>,
    pub pending_generic_calls: BTreeSet<ExprId>,
    pub generic_methods: BTreeMap<(String, String), String>,
    pub generic_call_names: BTreeMap<ExprId, String>,
    pub generic_call_types: BTreeMap<ExprId, Vec<Type>>,
    pub generic_constructors: BTreeMap<String, GenericConstructorInfo>,
    pub constructor_applications: BTreeMap<crate::generic::constructor::ConstructorKey, String>,
    pub generic_constructor_functions: BTreeSet<String>,
    pub generic_constructor_requests: std::cell::RefCell<
        BTreeMap<
            crate::generic::constructor::ConstructorKey,
            crate::generic::constructor::ConstructorRequest,
        >,
    >,
    pub infer_generic_constructors: bool,
    pub static_fields: BTreeMap<(String, String), String>,
    pub static_owners: BTreeMap<String, String>,
    pub nested_types: BTreeMap<(String, String), String>,
    pub type_owners: BTreeMap<String, String>,
    pub static_references: BTreeMap<ExprId, String>,
    pub static_type_lookups: BTreeSet<ExprId>,
    pub methods: BTreeMap<(String, String), String>,
    pub method_receivers: BTreeMap<ExprId, crate::method::MethodReceiver>,
    pub declaration_types: BTreeMap<SpanKey, Type>,
    pub matrix_layouts: BTreeMap<SpanKey, slang_layout::MatrixMajor>,
    pub diagnostics: Vec<Diagnostic>,
    pub return_type: Type,
    pub loops: usize,
    pub switches: usize,
    pub symbols: BTreeMap<String, SymbolOrigin>,
    pub current_module: ModuleId,
    pub public_types: BTreeSet<String>,
    pub public_fields: BTreeSet<(String, String)>,
    pub source_types: BTreeSet<(ModuleId, String)>,
    pub function_names: BTreeMap<SpanKey, String>,
    pub call_names: BTreeMap<ExprId, String>,
    pub call_output_parameters: BTreeMap<SpanKey, (String, Vec<bool>)>,
    pub global_references: BTreeMap<ExprId, SpanKey>,
    pub aliases: BTreeMap<String, TypeRef>,
    pub alias_types: BTreeMap<String, Type>,
    pub alias_matrix_layouts: BTreeMap<String, Option<slang_layout::MatrixMajor>>,
    pub resolving_aliases: BTreeSet<String>,
    pub struct_definitions: BTreeMap<String, Struct>,
    pub interface_definitions: BTreeMap<String, slang_syntax::Interface>,
    pub conformances: BTreeMap<String, BTreeSet<String>>,
    pub interface_witnesses: BTreeSet<SpanKey>,
    pub direct_witnesses: BTreeMap<crate::conformance::witness::WitnessKey, Signature>,
    pub symbolic_requirements: BTreeMap<(SpanKey, String), ConstraintCallInfo>,
    pub constraint_calls: BTreeMap<ExprId, ConstraintCallInfo>,
    pub constraint_variables: BTreeMap<SpanKey, ConstraintCallInfo>,
    pub shape_globals: BTreeMap<String, Variable>,
    pub shape_bindings: BTreeMap<String, Binding>,
    pub resolving_shape_globals: BTreeSet<String>,
    pub initializations: BTreeMap<ExprId, crate::Initialization>,
    pub default_arguments: BTreeMap<ExprId, Vec<crate::Initialization>>,
    pub case_values: BTreeMap<ExprId, i64>,
    pub resource_bindings: BTreeMap<SpanKey, slang_layout::ResourceBinding>,
    pub global_uniform_binding: Option<slang_layout::ResourceBinding>,
}

pub fn check(mut module: Module) -> Result<CheckedModule, Vec<Diagnostic>> {
    module.apply_legacy_visibility();
    check_with_metadata(
        module,
        vec![],
        BTreeMap::new(),
        slang_layout::MatrixMajor::Column,
    )
}

/// Check a program after imports and lexical module names have been resolved.
pub fn check_program(program: Program) -> Result<CheckedModule, Vec<Diagnostic>> {
    check_program_with_matrix_major(program, slang_layout::MatrixMajor::Column)
}

pub fn check_program_with_matrix_major(
    program: Program,
    major: slang_layout::MatrixMajor,
) -> Result<CheckedModule, Vec<Diagnostic>> {
    check_with_metadata(program.syntax, program.modules, program.symbols, major)
}

fn check_with_metadata(
    module: Module,
    modules: Vec<ModuleInfo>,
    symbols: BTreeMap<String, SymbolOrigin>,
    default_matrix_major: slang_layout::MatrixMajor,
) -> Result<CheckedModule, Vec<Diagnostic>> {
    let mut module = module;
    let bindings = crate::generic::validate_templates(&module, &symbols, default_matrix_major)?;
    module.constraint_calls = bindings.calls;
    module.constraint_variables = bindings.variables;
    let (module, symbols) = crate::generic::specialize(module, symbols, default_matrix_major)?;
    check_concrete(
        module,
        modules,
        symbols,
        default_matrix_major,
        BTreeSet::new(),
    )
}

pub(crate) fn check_concrete(
    module: Module,
    modules: Vec<ModuleInfo>,
    symbols: BTreeMap<String, SymbolOrigin>,
    default_matrix_major: slang_layout::MatrixMajor,
    symbolic_types: BTreeSet<String>,
) -> Result<CheckedModule, Vec<Diagnostic>> {
    check_concrete_inner(
        module,
        modules,
        symbols,
        default_matrix_major,
        symbolic_types,
        None,
    )
}

pub(crate) fn probe_generic_calls(
    module: Module,
    symbols: BTreeMap<String, SymbolOrigin>,
    major: slang_layout::MatrixMajor,
    symbolic_types: BTreeSet<String>,
    probe: &mut crate::generic::InferenceProbe,
) -> Result<CheckedModule, Vec<Diagnostic>> {
    check_concrete_inner(module, vec![], symbols, major, symbolic_types, Some(probe))
}

fn check_concrete_inner(
    mut module: Module,
    modules: Vec<ModuleInfo>,
    symbols: BTreeMap<String, SymbolOrigin>,
    default_matrix_major: slang_layout::MatrixMajor,
    symbolic_types: BTreeSet<String>,
    probe: Option<&mut crate::generic::InferenceProbe>,
) -> Result<CheckedModule, Vec<Diagnostic>> {
    if let Some(import) = module.imports.first() {
        return Err(vec![Diagnostic::error(
            "T001",
            import.span,
            "imports must be resolved before semantic checking",
        )]);
    }
    if let Some(include) = module.file_includes.first() {
        return Err(vec![Diagnostic::error(
            "T001",
            include.span,
            "module members must be loaded before semantic checking",
        )]);
    }
    let mut checker = Checker::new(&module, symbols, default_matrix_major);
    let probing = probe.is_some();
    checker.symbolic_types = symbolic_types;
    if let Some(probe) = &probe {
        checker.infer_generic_constructors = true;
        checker.pending_generic_calls = probe.pending.clone();
        checker.generic_methods = probe.methods.clone();
    }
    checker.declare(&module);
    checker.declare_conformances(&module);
    checker.declare_function_applications(&module);
    let global_declarations: BTreeMap<_, _> = module
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Global(v) => Some((v.name.as_str(), v)),
            _ => None,
        })
        .collect();
    let mut buffer_names = BTreeSet::new();
    for buffer in &module.constant_buffers {
        let owner = checker
            .symbols
            .get(&buffer.variable_name)
            .map_or(ModuleId(0), |origin| origin.module);
        if !buffer_names.insert((owner, buffer.name.as_str())) {
            checker.error(
                "T003",
                buffer.span,
                format!("duplicate cbuffer '{}'", buffer.name),
            );
        }
        let Some(handle) = global_declarations.get(buffer.variable_name.as_str()) else {
            continue;
        };
        if !handle.modifiers.iter().any(|m| m == "public") {
            for member in buffer.members.iter().filter(|m| m.static_storage) {
                if global_declarations
                    .get(member.variable_name.as_str())
                    .is_some_and(|field| field.modifiers.iter().any(|m| m == "public"))
                {
                    checker.error(
                        "T018",
                        member.span,
                        "a public cbuffer field requires a public cbuffer",
                    );
                }
            }
        }
    }
    for declaration in &module.declarations {
        if let Declaration::Global(variable) = declaration {
            if probe
                .as_ref()
                .is_some_and(|p| p.deferred.contains(&span_key(variable.span)))
            {
                continue;
            }
            checker.current_module = checker
                .symbols
                .get(&variable.name)
                .map_or(ModuleId(0), |origin| origin.module);
            checker.current_static_type = checker.static_owners.get(&variable.name).cloned();
            checker.current_type = checker.current_static_type.clone();
            checker.initialize_global(variable);
            checker.current_type = None;
            checker.current_static_type = None;
        }
    }
    checker.resolve_global_constants(&module);
    let mut entry_points = vec![];
    for declaration in &module.declarations {
        let name = match declaration {
            Declaration::Struct(s) => &s.name,
            Declaration::Interface(i) => &i.name,
            Declaration::Function(f) => &f.name,
            Declaration::Global(v) => &v.name,
            Declaration::TypeAlias(a) => &a.name,
        };
        checker.current_module = checker
            .symbols
            .get(name)
            .map_or(ModuleId(0), |origin| origin.module);
        match declaration {
            Declaration::TypeAlias(_) => {}
            Declaration::Interface(interface) => {
                if probe
                    .as_ref()
                    .is_some_and(|p| p.deferred.contains(&span_key(interface.span)))
                {
                    continue;
                }
                for requirement in &interface.requirements {
                    checker.check_constraint_defaults(&ConstraintCallInfo {
                        owner: crate::conformance::witness::reference(
                            interface.name.clone(),
                            interface.span,
                        ),
                        interface: crate::conformance::witness::reference(
                            interface.name.clone(),
                            interface.span,
                        ),
                        requirement: requirement.clone(),
                        declaration_module: checker.current_module.0,
                    });
                }
            }
            Declaration::Global(_) => {}
            Declaration::Function(function) => {
                if probe
                    .as_ref()
                    .is_some_and(|p| p.deferred.contains(&span_key(function.span)))
                {
                    continue;
                }
                if let Some(entry) = checker.entry_point(function, &module) {
                    entry_points.push(entry);
                }
                checker.function(function);
            }
            Declaration::Struct(structure) => {
                if probe
                    .as_ref()
                    .is_some_and(|p| p.deferred.contains(&span_key(structure.span)))
                {
                    continue;
                }
                checker.current_static_type = Some(structure.name.clone());
                for field in &structure.fields {
                    if field.modifiers.iter().any(|m| {
                        !matches!(
                            m.as_str(),
                            "nointerpolation"
                                | "public"
                                | "internal"
                                | "row_major"
                                | "column_major"
                        )
                    }) {
                        checker.error(
                            "T001",
                            field.span,
                            "unsupported struct field initializer or modifier",
                        );
                    }
                    checker.check_interpolation(&field.modifiers, field.span);
                    let ty = checker.declaration_types[&span_key(field.span)].clone();
                    checker.check_matrix_layout(&ty, &field.ty, &field.modifiers, field.span);
                    checker.check_visibility_modifiers(&field.modifiers, field.span);
                    if field.initializer.is_some() {
                        checker.initializer(field);
                    }
                }
                checker.current_static_type = None;
            }
        }
    }
    checker.validate_constant_buffers();
    checker.validate_static_initializers(&module);
    checker.check_binding_overlaps(&module);
    checker.allocate_resource_bindings(&module);
    if let Some(probe) = probe {
        probe.types = std::mem::take(&mut checker.generic_call_types);
        probe.shapes = checker.symbolic_shapes.clone();
        probe.record_selections(&checker);
    }
    if !checker
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        let global_constants = checker.scopes[0]
            .values()
            .filter_map(|binding| {
                if let Storage::Global(key) = binding.storage
                    && matches!(
                        binding.ty,
                        Type::Scalar(
                            crate::Scalar::Int16
                                | crate::Scalar::Uint16
                                | crate::Scalar::Int
                                | crate::Scalar::Uint
                                | crate::Scalar::Int64
                                | crate::Scalar::Uint64
                                | crate::Scalar::IntPtr
                                | crate::Scalar::UintPtr
                                | crate::Scalar::Bool
                        )
                    )
                {
                    binding.constant.map(|value| (key, value))
                } else {
                    None
                }
            })
            .collect();
        module.constraint_calls = checker.constraint_calls;
        module.constraint_variables = checker
            .constraint_variables
            .into_iter()
            .map(
                |((source, start, end, context), call)| ConstraintVariableInfo {
                    span: Span {
                        source: slang_source::SourceId(source),
                        start,
                        end,
                        context,
                    },
                    call,
                },
            )
            .collect();
        let mut checked = CheckedModule {
            matrix_major: checker.default_matrix_major,
            module,
            expression_types: checker.expression_types,
            constant_buffer_loads: checker.constant_buffer_loads,
            swizzle_indices: checker.swizzle_indices,
            implicit_members: checker.implicit_members,
            method_receivers: checker.method_receivers,
            static_references: checker.static_references,
            static_type_lookups: checker.static_type_lookups,
            variable_initializations: checker.variable_initializations,
            constructor_initializations: checker.constructor_initializations,
            declaration_types: checker.declaration_types,
            matrix_layouts: checker.matrix_layouts,
            entry_points,
            modules,
            symbols: checker.symbols,
            function_names: checker.function_names,
            call_names: checker.call_names,
            call_output_parameters: checker.call_output_parameters,
            global_references: checker.global_references,
            initializations: checker.initializations,
            default_arguments: checker.default_arguments,
            case_values: checker.case_values,
            entry_point_requirements: BTreeMap::new(),
            global_constants,
            resource_bindings: checker.resource_bindings,
            global_uniform_binding: checker.global_uniform_binding,
            diagnostics: checker.diagnostics,
        };
        if !probing {
            checked.entry_point_requirements = crate::capability::validate(&checked)?;
        }
        Ok(checked)
    } else {
        Err(checker.diagnostics)
    }
}

impl Checker {
    pub(crate) fn new(
        module: &Module,
        symbols: BTreeMap<String, SymbolOrigin>,
        default_matrix_major: slang_layout::MatrixMajor,
    ) -> Self {
        Self {
            default_matrix_major,
            scopes: vec![BTreeMap::new()],
            functions: BTreeMap::new(),
            structures: BTreeMap::new(),
            expression_types: BTreeMap::new(),
            constant_buffer_loads: BTreeSet::new(),
            swizzle_indices: BTreeMap::new(),
            implicit_members: BTreeMap::new(),
            variable_initializations: BTreeMap::new(),
            constructor_initializations: BTreeMap::new(),
            constructors: BTreeMap::new(),
            current_constructor: None,
            current_type: None,
            current_static_type: None,
            generic_type_contexts: module
                .generic_instances
                .iter()
                .map(|i| (i.context, crate::generic::argument_names(&i.arguments)))
                .collect(),
            symbolic_types: BTreeSet::new(),
            symbolic_shapes: BTreeMap::new(),
            pending_generic_calls: BTreeSet::new(),
            generic_methods: BTreeMap::new(),
            generic_call_names: BTreeMap::new(),
            generic_call_types: BTreeMap::new(),
            generic_constructors: module
                .generic_constructors
                .iter()
                .map(|c| (c.owner.clone(), c.clone()))
                .collect(),
            constructor_applications: module
                .constructor_applications
                .iter()
                .map(|c| {
                    (
                        (c.owner.clone(), c.argument_types.clone()),
                        c.function.clone(),
                    )
                })
                .collect(),
            generic_constructor_functions: module
                .generic_instances
                .iter()
                .filter(|i| {
                    module
                        .generic_constructors
                        .iter()
                        .any(|c| c.function == i.template_name)
                })
                .map(|i| i.type_name.clone())
                .collect(),
            generic_constructor_requests: Default::default(),
            infer_generic_constructors: false,
            static_fields: module
                .static_members
                .iter()
                .map(|m| ((m.owner.clone(), m.name.clone()), m.variable_name.clone()))
                .collect(),
            static_owners: module
                .static_members
                .iter()
                .map(|m| (m.variable_name.clone(), m.owner.clone()))
                .collect(),
            static_references: BTreeMap::new(),
            static_type_lookups: BTreeSet::new(),
            nested_types: module
                .nested_types
                .iter()
                .map(|m| ((m.owner.clone(), m.name.clone()), m.type_name.clone()))
                .collect(),
            type_owners: module
                .nested_types
                .iter()
                .map(|m| (m.type_name.clone(), m.owner.clone()))
                .collect(),
            methods: BTreeMap::new(),
            method_receivers: BTreeMap::new(),
            declaration_types: BTreeMap::new(),
            matrix_layouts: BTreeMap::new(),
            diagnostics: vec![],
            return_type: Type::Void,
            loops: 0,
            switches: 0,
            symbols,
            current_module: ModuleId(0),
            public_types: BTreeSet::new(),
            public_fields: BTreeSet::new(),
            source_types: BTreeSet::new(),
            function_names: BTreeMap::new(),
            call_names: BTreeMap::new(),
            call_output_parameters: BTreeMap::new(),
            global_references: BTreeMap::new(),
            aliases: BTreeMap::new(),
            alias_types: BTreeMap::new(),
            alias_matrix_layouts: BTreeMap::new(),
            resolving_aliases: BTreeSet::new(),
            struct_definitions: BTreeMap::new(),
            interface_definitions: BTreeMap::new(),
            conformances: BTreeMap::new(),
            interface_witnesses: BTreeSet::new(),
            direct_witnesses: BTreeMap::new(),
            symbolic_requirements: BTreeMap::new(),
            constraint_calls: module.constraint_calls.clone(),
            constraint_variables: module
                .constraint_variables
                .iter()
                .map(|v| (span_key(v.span), v.call.clone()))
                .collect(),
            shape_globals: module
                .declarations
                .iter()
                .filter_map(|d| {
                    if let Declaration::Global(v) = d {
                        Some((v.name.clone(), v.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
            shape_bindings: BTreeMap::new(),
            resolving_shape_globals: BTreeSet::new(),
            initializations: BTreeMap::new(),
            default_arguments: BTreeMap::new(),
            case_values: BTreeMap::new(),
            resource_bindings: BTreeMap::new(),
            global_uniform_binding: None,
        }
    }

    pub(crate) fn reference_matrix_major(
        &mut self,
        reference: &TypeRef,
    ) -> Option<slang_layout::MatrixMajor> {
        reference.matrix_major.or_else(|| {
            let name = self.type_name(&reference.name, reference.span)?;
            self.alias_matrix_layouts.get(&name).copied().flatten()
        })
    }

    pub(crate) fn check_matrix_layout(
        &mut self,
        ty: &Type,
        reference: &TypeRef,
        modifiers: &[String],
        span: Span,
    ) {
        let major = modifiers
            .iter()
            .rev()
            .find_map(|m| match m.as_str() {
                "row_major" => Some(slang_layout::MatrixMajor::Row),
                "column_major" => Some(slang_layout::MatrixMajor::Column),
                _ => None,
            })
            .or_else(|| self.reference_matrix_major(reference));
        if let Some(major) = major {
            self.check_matrix_type(ty, span);
            self.matrix_layouts.insert(span_key(span), major);
        }
    }

    pub(crate) fn check_matrix_type(&mut self, mut ty: &Type, span: Span) {
        while let Type::Array(element, _) = ty {
            ty = element;
        }
        let symbolic_matrix = matches!(ty, Type::Struct(name) if self.symbolic_shapes.get(name).is_some_and(|(_, d)| d.len() == 2));
        if !symbolic_matrix && !matches!(ty, Type::Matrix(..) | Type::Error) {
            self.error(
                "T001",
                span,
                "matrix layout modifiers require a matrix or an array of matrices",
            );
        }
    }

    pub fn error(&mut self, code: &str, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(code, span, message));
    }

    pub fn require_conversion(&mut self, actual: &Type, expected: &Type, span: Span) {
        if !actual.convertible_to(expected) {
            self.error(
                "T008",
                span,
                format!("cannot convert {actual} to {expected}"),
            );
        }
    }

    pub fn condition(&mut self, expression: &Expr) {
        let ty = self.value_expression(expression);
        if !matches!(ty, Type::Scalar(_) | Type::Vector(_, 1) | Type::Error) {
            self.error("T009", expression.span, "condition must have a scalar type");
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, scope)| {
                if index == 0 && !self.module_symbol_visible(name) {
                    None
                } else {
                    scope.get(name)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slang_source::SourceId;
    fn run(text: &str) -> Result<CheckedModule, Vec<Diagnostic>> {
        check(slang_syntax::parse(&slang_lexer::lex(SourceId(0), text).tokens).unwrap())
    }

    #[test]
    fn compute_resource_access_and_overloads() {
        let text = "float twice(float x){return x*2.0;} int twice(int x){return x*2;} RWStructuredBuffer<float> output; [shader(\"compute\")][numthreads(64,1,1)] void main(uint3 id:SV_DispatchThreadID){output[id.x]=twice(float(id.x));}";
        let module = run(text).unwrap();
        assert_eq!(module.entry_points[0].thread_group_size, Some([64, 1, 1]));
    }

    #[test]
    fn invalid_programs_are_rejected() {
        for text in [
            "void f(){missing=1;}",
            "void f(){const int x=1;x=2;}",
            "void f(){break;}",
            "int f(){if(true)return 1;}",
            "StructuredBuffer<float> b; void f(){b[0]=1;}",
            "void f(){float2 x;x.z=1;}",
            "void f(){float2 x;x.xx=float2(1,2);}",
            "[shader(\"compute\")][numthreads(0,1,1)]void main(){}",
            "void f(){int x;{int y;} x=y;}",
            "struct A { A x; };",
            "void f(out int a){} void g(){f(1);}",
        ] {
            assert!(run(text).is_err(), "{text}");
        }
    }

    #[test]
    fn globals_and_parameter_qualifiers() {
        assert!(run("static float x=1.0; void f(){x=2.0;}").is_ok());
        assert!(run("float x; void f(){x=2.0;}").is_err());
        assert!(run("void f(){groupshared int x;}").is_err());
        assert!(run("void f(const out int x){}").is_err());
        assert!(run("void f(in out int x){}").is_err());
        assert!(
            slang_syntax::parse(&slang_lexer::lex(SourceId(0), "void f(){int return;}").tokens)
                .is_err()
        );
    }

    #[test]
    fn resolved_global_references_preserve_shadowing_without_module_linking() {
        for text in [
            "groupshared int value;[shader(\"fragment\")]float4 main():SV_Target{int value=1;return float4(value);}",
            "float read(){discard;return 1;}static float value=read();[shader(\"compute\")][numthreads(1,1,1)]void main(){float value=2;float copy=value;}",
        ] {
            let checked = run(text).unwrap();
            assert!(checked.global_references.is_empty());
        }
        let checked = run("groupshared int value;[shader(\"compute\")][numthreads(1,1,1)]void main(){int copy=value;}").unwrap();
        assert_eq!(checked.global_references.len(), 1);
        assert!(run("groupshared int value;[shader(\"fragment\")]float4 main():SV_Target{return float4(value);}").is_err());
    }
}
