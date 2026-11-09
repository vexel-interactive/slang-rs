//! Infer template arguments from ordinary semantic expression types.

use super::*;
use crate::Type;

impl Specializer {
    fn publish_metadata(&self, module: &mut Module) {
        module.nested_types = self.nested_output.clone();
        module.static_members = self.static_output.clone();
        module.generic_instances = self.instance_output.clone();
        module.function_applications = self.application_output.clone();
        module.generic_constructors = self.constructor_templates.values().cloned().collect();
        module.constructor_applications = self
            .constructor_applications
            .iter()
            .map(|((owner, types), function)| ConstructorApplicationInfo {
                owner: owner.clone(),
                argument_types: types.clone(),
                function: function.clone(),
            })
            .collect();
    }

    pub(super) fn infer_calls(
        &mut self,
        module: &mut Module,
        major: slang_layout::MatrixMajor,
    ) -> Result<(), Diagnostic> {
        self.publish_metadata(module);
        for _ in 0..128 {
            let mut changed = self.activate_referenced(module)?;
            changed |= self.drain_pending(module)?;
            self.publish_metadata(module);
            let mut probe = InferenceProbe {
                pending: self.inference_calls.keys().copied().collect(),
                deferred: self.generated.difference(&self.active).copied().collect(),
                methods: self.method_templates.clone(),
                ..Default::default()
            };
            let checked = crate::checker::probe_generic_calls(
                module.clone(),
                self.symbols.clone(),
                major,
                self.symbolic_types.clone(),
                &mut probe,
            );
            let mut targets = BTreeMap::new();
            let mut failures = vec![];
            for (key, request) in &probe.constructor_requests {
                if request.types.iter().any(contains_error) {
                    continue;
                }
                let call = InferenceCall {
                    name: self.constructor_templates[&key.0].function.clone(),
                    supplied: vec![],
                    explicit: false,
                    span: request.span,
                    module: request.module,
                };
                self.shapes.current_module = request.module;
                let arguments = request
                    .types
                    .iter()
                    .map(|t| type_reference(t, &probe.shapes, request.span))
                    .collect::<Vec<_>>();
                let target = self.infer_application(&call, &arguments, true)?;
                self.constructor_applications.insert(key.clone(), target);
                changed = true;
            }
            changed |= self.activate_selected(module, &probe)?;
            for (id, types) in &probe.types {
                if types.iter().any(contains_error) {
                    continue;
                }
                let mut call = self.inference_calls.remove(id).unwrap();
                if let Some(target) = probe.targets.get(id) {
                    call.name = target.clone();
                }
                self.shapes.current_module = call.module;
                let arguments = types
                    .iter()
                    .map(|t| type_reference(t, &probe.shapes, call.span))
                    .collect::<Vec<_>>();
                match self.infer_application(&call, &arguments, false) {
                    Ok(target) => {
                        targets.insert(*id, target);
                    }
                    Err(diagnostic) => {
                        failures.push(diagnostic);
                        self.inference_calls.insert(*id, call);
                    }
                }
            }
            changed |= !targets.is_empty();
            if !changed {
                if self.inference_calls.is_empty() {
                    if let Some(diagnostic) = probe
                        .ambiguities
                        .into_iter()
                        .find(|d| self.active_diagnostic(module, d))
                    {
                        return Err(diagnostic);
                    }
                    if let Err(diagnostics) = checked
                        && let Some(diagnostic) = diagnostics.into_iter().find(|d| {
                            d.severity == slang_source::Severity::Error
                                && self.active_diagnostic(module, d)
                        })
                    {
                        return Err(diagnostic);
                    }
                    self.prune_dormant(module);
                    return Ok(());
                }
                if let Some(diagnostic) = failures.into_iter().next() {
                    return Err(diagnostic);
                }
                if let Err(diagnostics) = checked
                    && let Some(diagnostic) = diagnostics.into_iter().find(|d| {
                        d.severity == slang_source::Severity::Error
                            && self.active_diagnostic(module, d)
                    })
                {
                    return Err(diagnostic);
                }
                return Err(error(
                    self.inference_calls.values().next().unwrap().span,
                    "cannot infer generic function arguments",
                ));
            }
            for declaration in &mut module.declarations {
                CallTargets(&targets).declaration(declaration)?;
            }
            self.drain_pending(module)?;
            self.publish_metadata(module);
        }
        Err(error(
            self.inference_calls.values().next().map_or_else(
                || module.declarations.first().map_or_default(declaration_span),
                |c| c.span,
            ),
            "generic inference or activation limit exceeded",
        ))
    }

    fn infer_application(
        &mut self,
        call: &InferenceCall,
        arguments: &[Option<TypeRef>],
        allow_empty: bool,
    ) -> Result<String, Diagnostic> {
        let mut candidates = vec![];
        if !call.explicit && self.ordinary_functions.contains(&call.name) {
            candidates.push(FunctionCandidateInfo {
                function: call.name.clone(),
                default_type_arguments: 0,
                generic: false,
            });
        }
        for key in self
            .function_templates
            .get(&call.name)
            .cloned()
            .unwrap_or_default()
        {
            let template = self.templates[&key].clone();
            self.shapes.current_module = self
                .symbols
                .get(&template.name)
                .map_or(Default::default(), |o| o.module);
            if call.supplied.len() > template.parameters.len() {
                continue;
            }
            let names = template
                .parameters
                .iter()
                .map(|p| p.name.clone())
                .collect::<BTreeSet<_>>();
            let explicit = template.parameters[..call.supplied.len()]
                .iter()
                .map(|p| p.name.clone())
                .collect::<BTreeSet<_>>();
            let mut substitutions = template
                .parameters
                .iter()
                .zip(&call.supplied)
                .map(|(p, a)| {
                    let TypeArgument::Type(a) = a else {
                        unreachable!()
                    };
                    (p.name.clone(), a.clone())
                })
                .collect::<BTreeMap<_, _>>();
            let Declaration::Function(function) = &template.declarations[0] else {
                unreachable!()
            };
            let mut valid = true;
            let offset = usize::from(function.method.as_ref().is_some_and(|m| !m.is_static));
            for (parameter, argument) in function.parameters[offset..].iter().zip(arguments) {
                let mut pattern = parameter.ty.clone();
                self.signature_type(&mut pattern, &names, 0)?;
                let Some(argument) = argument else {
                    if pattern.base.is_none()
                        && pattern.arguments.is_empty()
                        && names.contains(&pattern.name)
                        && !explicit.contains(&pattern.name)
                    {
                        valid = false;
                        break;
                    }
                    continue;
                };
                if !self.match_argument(&pattern, argument, &names, &explicit, &mut substitutions) {
                    valid = false;
                    break;
                }
            }
            let mut supplied = vec![];
            let mut defaults = 0;
            for parameter in &template.parameters {
                let Some(mut argument) =
                    substitutions.get(&parameter.name).cloned().or_else(|| {
                        defaults += 1;
                        parameter.default.clone()
                    })
                else {
                    valid = false;
                    break;
                };
                substitute::replace_type(&mut argument, &substitutions);
                self.canonical(&mut argument, 0)?;
                self.validate_argument(&mut argument)?;
                substitutions.insert(parameter.name.clone(), argument.clone());
                supplied.push(TypeArgument::Type(argument));
            }
            if !valid || !self.valid_signature(function, &substitutions)? {
                continue;
            }
            candidates.push(FunctionCandidateInfo {
                function: self.instantiate(&key, &supplied, call.span)?,
                default_type_arguments: defaults,
                generic: true,
            });
        }
        if candidates.is_empty() && !allow_empty {
            return Err(error(
                call.span,
                "no generic overload can infer the required type arguments",
            ));
        }
        if self.application_output.len() >= 1024 {
            return Err(error(call.span, "generic specialization limit exceeded"));
        }
        let target = format!("{}function_call_{}", self.prefix, self.next_context);
        self.next_context += 1;
        if let Some(origin) = self.symbols.get(&call.name).cloned() {
            self.symbols.insert(target.clone(), origin);
        }
        self.application_output.push(FunctionApplicationInfo {
            name: target.clone(),
            candidates,
        });
        Ok(target)
    }

    fn valid_signature(
        &mut self,
        function: &Function,
        substitutions: &BTreeMap<String, TypeRef>,
    ) -> Result<bool, Diagnostic> {
        self.shapes.symbolic_types = self.symbolic_types.clone();
        // This preflight checks type structure. The final checker enforces visibility.
        for reference in function
            .parameters
            .iter()
            .map(|p| &p.ty)
            .chain(std::iter::once(&function.return_type))
        {
            let mut reference = reference.clone();
            SignatureSubstitutions(substitutions).ty(&mut reference)?;
            self.canonical(&mut reference, 0)?;
            let count = self.shapes.diagnostics.len();
            let owner = self.shapes.current_module;
            self.shapes.current_module = Default::default();
            let ty = self.shapes.resolve_type(&reference);
            self.shapes.current_module = owner;
            let valid = !contains_error(&ty)
                && !self.shapes.diagnostics[count..]
                    .iter()
                    .any(|d| d.severity == slang_source::Severity::Error);
            self.shapes.diagnostics.truncate(count);
            if !valid {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn match_argument(
        &mut self,
        pattern: &TypeRef,
        actual: &TypeRef,
        names: &BTreeSet<String>,
        explicit: &BTreeSet<String>,
        substitutions: &mut BTreeMap<String, TypeRef>,
    ) -> bool {
        let mut pattern = pattern.clone();
        let mut actual = actual.clone();
        if actual.name == "ConstantBuffer"
            && actual.arrays.is_empty()
            && pattern.name != "ConstantBuffer"
            && !(pattern.base.is_none()
                && pattern.arguments.is_empty()
                && pattern.arrays.is_empty()
                && names.contains(&pattern.name))
            && let Some(TypeArgument::Type(element)) = actual.arguments.first()
        {
            return self.match_argument(&pattern, element, names, explicit, substitutions);
        }
        if actual.arrays.len() < pattern.arrays.len() {
            return false;
        }
        for (a, b) in pattern.arrays.iter().zip(&actual.arrays) {
            if self.shape_value(a) != self.shape_value(b) {
                return false;
            }
        }
        actual.arrays.drain(..pattern.arrays.len());
        pattern.arrays.clear();
        if pattern.base.is_none() && pattern.arguments.is_empty() && names.contains(&pattern.name) {
            if explicit.contains(&pattern.name) {
                return true;
            }
            let argument = if let Some(previous) = substitutions.get(&pattern.name) {
                if canonical::type_key(previous) == canonical::type_key(&actual) {
                    return true;
                }
                let Some(a) = numeric_type(previous) else {
                    return false;
                };
                let Some(b) = numeric_type(&actual) else {
                    return false;
                };
                let Some(common) = Type::inferred_common(&a, &b) else {
                    return false;
                };
                type_reference(&common, &BTreeMap::new(), actual.span).unwrap()
            } else {
                actual
            };
            substitutions.insert(pattern.name, argument);
            return true;
        }
        if !argument_names(std::slice::from_ref(&pattern))
            .iter()
            .any(|n| names.contains(n))
        {
            return true;
        }
        if !actual.arrays.is_empty() {
            return false;
        }
        if matches!(pattern.name.as_str(), "vector" | "matrix") {
            let Some((component, dimensions)) = numeric_parts(&actual) else {
                return false;
            };
            let count = if pattern.name == "vector" { 1 } else { 2 };
            let mut expected = pattern
                .arguments
                .iter()
                .skip(1)
                .map(|a| match a {
                    TypeArgument::Integer { value, .. } => Some(*value),
                    TypeArgument::Expression(e) => {
                        self.shape_value(e).and_then(|n| u32::try_from(n).ok())
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
            expected.resize(count, 4);
            if !dimensions.is_empty() && dimensions != expected {
                return false;
            }
            return match pattern.arguments.first() {
                Some(TypeArgument::Type(parameter)) => {
                    self.match_argument(parameter, &component, names, explicit, substitutions)
                }
                _ => true,
            };
        }
        if let Some(base) = &pattern.base {
            let member = if let Some(instance) = self
                .instance_output
                .iter()
                .find(|i| i.type_name == actual.name)
            {
                self.templates
                    .get(&instance.template_name)
                    .and_then(|t| t.owner.clone())
            } else {
                self.nested_output
                    .iter()
                    .find(|n| n.type_name == actual.name)
                    .cloned()
            };
            let Some(member) = member.filter(|m| m.name == pattern.name) else {
                return false;
            };
            if !self.match_argument(
                base,
                &named_reference(member.owner, actual.span),
                names,
                explicit,
                substitutions,
            ) {
                return false;
            }
        }
        let actual_arguments = if let Some(instance) = self
            .instance_output
            .iter()
            .find(|i| i.type_name == actual.name)
        {
            if pattern.base.is_none() && instance.template_name != pattern.name {
                return false;
            }
            instance.arguments[..instance.parameter_count]
                .iter()
                .cloned()
                .map(TypeArgument::Type)
                .collect::<Vec<_>>()
        } else {
            if pattern.base.is_none() && pattern.name != actual.name {
                return false;
            }
            actual.arguments.clone()
        };
        let omitted_defaults = self
            .instance_output
            .iter()
            .find(|i| i.type_name == actual.name)
            .and_then(|i| self.templates.get(&i.template_name))
            .is_some_and(|t| {
                pattern.arguments.len() <= t.parameters.len()
                    && t.parameters[pattern.arguments.len()..]
                        .iter()
                        .all(|p| p.default.is_some())
            });
        if pattern.arguments.len() != actual_arguments.len() && !omitted_defaults {
            return false;
        }
        pattern
            .arguments
            .iter()
            .zip(&actual_arguments)
            .all(|(a, b)| match (a, b) {
                (TypeArgument::Type(a), TypeArgument::Type(b)) => {
                    self.match_argument(a, b, names, explicit, substitutions)
                }
                _ => false,
            })
    }
}

struct CallTargets<'a>(&'a BTreeMap<ExprId, String>);
impl VisitorMut for CallTargets<'_> {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let Some(target) = self.0.get(&expression.id) {
            if let ExprKind::GenericApplication { callee, .. } = &expression.kind {
                expression.kind = callee.kind.clone();
            }
            if let ExprKind::Member { name, .. } = &mut expression.kind {
                *name = target.clone();
            } else {
                expression.kind = ExprKind::Name(target.clone());
            }
            return Ok(());
        }
        visit::walk_expression(self, expression)
    }
}

struct SignatureSubstitutions<'a>(&'a BTreeMap<String, TypeRef>);
impl VisitorMut for SignatureSubstitutions<'_> {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        substitute::replace_type(ty, self.0);
        visit::walk_type(self, ty)
    }
}

fn named_reference(name: String, span: Span) -> TypeRef {
    TypeRef {
        name,
        base: None,
        application: false,
        owner_ref: false,
        parameter_ref: false,
        arguments: vec![],
        matrix_major: None,
        arrays: vec![],
        span,
    }
}

fn type_reference(
    ty: &Type,
    shapes: &BTreeMap<String, (Type, Vec<u8>)>,
    span: Span,
) -> Option<TypeRef> {
    let mut reference = named_reference(ty.to_string(), span);
    match ty {
        Type::Error | Type::InitializerList | Type::Callable { .. } | Type::Meta(_) => return None,
        Type::Array(element, count) => {
            reference = type_reference(element, shapes, span)?;
            reference
                .arrays
                .insert(0, Expr::integer(ExprId(0), *count, span));
        }
        Type::Vector(s, n) if s.is_pointer_sized() => {
            return numeric_reference(Type::Scalar(*s), &[*n], shapes, span);
        }
        Type::Matrix(s, r, c) if s.is_pointer_sized() => {
            return numeric_reference(Type::Scalar(*s), &[*r, *c], shapes, span);
        }
        Type::Struct(name) if shapes.contains_key(name) => {
            let (component, dimensions) = &shapes[name];
            return numeric_reference(component.clone(), dimensions, shapes, span);
        }
        Type::ConstantBuffer {
            element,
            matrix_major,
        }
        | Type::Buffer {
            element,
            matrix_major,
            ..
        } => {
            reference.name = match ty {
                Type::ConstantBuffer { .. } => "ConstantBuffer",
                Type::Buffer { writable: true, .. } => "RWStructuredBuffer",
                _ => "StructuredBuffer",
            }
            .into();
            let mut element = type_reference(element, shapes, span)?;
            element.matrix_major = *matrix_major;
            reference.arguments.push(TypeArgument::Type(element));
            reference.application = true;
        }
        Type::Texture {
            element,
            dimension,
            writable,
        } => {
            reference.name = format!(
                "{}Texture{}",
                if *writable { "RW" } else { "" },
                match dimension {
                    crate::TextureDimension::D1 => "1D",
                    crate::TextureDimension::D2 => "2D",
                    crate::TextureDimension::D3 => "3D",
                    crate::TextureDimension::Cube => "Cube",
                }
            );
            reference
                .arguments
                .push(TypeArgument::Type(type_reference(element, shapes, span)?));
            reference.application = true;
        }
        _ => {}
    }
    Some(reference)
}

fn numeric_reference(
    component: Type,
    dimensions: &[u8],
    shapes: &BTreeMap<String, (Type, Vec<u8>)>,
    span: Span,
) -> Option<TypeRef> {
    let mut reference = named_reference(
        if dimensions.len() == 1 {
            "vector"
        } else {
            "matrix"
        }
        .into(),
        span,
    );
    reference.application = true;
    reference.arguments.push(TypeArgument::Type(type_reference(
        &component, shapes, span,
    )?));
    reference
        .arguments
        .extend(dimensions.iter().map(|&n| TypeArgument::Integer {
            value: u32::from(n),
            span,
        }));
    Some(reference)
}

fn numeric_parts(reference: &TypeRef) -> Option<(TypeRef, Vec<u32>)> {
    if let Some(ty) = Type::builtin(&reference.name) {
        let dimensions = match ty {
            Type::Scalar(_) => vec![],
            Type::Vector(_, n) => vec![u32::from(n)],
            Type::Matrix(_, r, c) => vec![u32::from(r), u32::from(c)],
            _ => return None,
        };
        return Some((
            named_reference(ty.scalar()?.to_string(), reference.span),
            dimensions,
        ));
    }
    if matches!(reference.name.as_str(), "vector" | "matrix") {
        let Some(TypeArgument::Type(component)) = reference.arguments.first() else {
            return None;
        };
        let dimensions = reference
            .arguments
            .iter()
            .skip(1)
            .map(|a| match a {
                TypeArgument::Integer { value, .. } => Some(*value),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        return Some((component.clone(), dimensions));
    }
    None
}

fn numeric_type(reference: &TypeRef) -> Option<Type> {
    if !reference.arrays.is_empty() {
        return None;
    }
    let (component, dimensions) = numeric_parts(reference)?;
    let Type::Scalar(scalar) = Type::builtin(&component.name)? else {
        return None;
    };
    match dimensions.as_slice() {
        [] => Some(Type::Scalar(scalar)),
        [n] => Some(Type::Vector(scalar, u8::try_from(*n).ok()?)),
        [r, c] => Some(Type::Matrix(
            scalar,
            u8::try_from(*r).ok()?,
            u8::try_from(*c).ok()?,
        )),
        _ => None,
    }
}

fn contains_error(ty: &Type) -> bool {
    match ty {
        Type::Error => true,
        Type::Array(t, _)
        | Type::ConstantBuffer { element: t, .. }
        | Type::Buffer { element: t, .. }
        | Type::Texture { element: t, .. } => contains_error(t),
        _ => false,
    }
}
