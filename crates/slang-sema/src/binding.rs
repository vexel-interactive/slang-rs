use crate::{Type, checker::Checker, declaration::VariableScope, span_key};
use slang_layout::{
    AllocationError, MAX_REGISTER_INDEX, RegisterAllocator, RegisterClass, ResourceBinding,
};
use slang_source::{Diagnostic, Severity};
use slang_syntax::{Declaration, Module, Variable};

impl Checker {
    pub(crate) fn check_register(&mut self, variable: &Variable, ty: &Type, scope: VariableScope) {
        let Some(annotation) = variable.register else {
            return;
        };
        if !matches!(scope, VariableScope::Global)
            || variable
                .modifiers
                .iter()
                .any(|m| matches!(m.as_str(), "static" | "groupshared"))
        {
            self.error(
                "T022",
                annotation.span,
                "register annotations currently require a global shader resource",
            );
            return;
        }
        let (class, count) = match resource_range(ty) {
            Ok(Some(range)) => range,
            Ok(None) if *ty == Type::Error => return,
            Ok(None) => {
                self.error(
                    "T022",
                    annotation.span,
                    "register annotations on numeric or aggregate uniforms are not implemented",
                );
                return;
            }
            Err(error) => {
                self.error("T022", annotation.span, error.to_string());
                return;
            }
        };
        if annotation.binding.class != class {
            self.error(
                "T022",
                annotation.span,
                format!("type '{ty}' requires register class '{class}'"),
            );
            return;
        }
        if self.is_empty_constant_buffer(ty) {
            return;
        }
        let binding = ResourceBinding {
            register: annotation.binding,
            count,
            explicit: true,
        };
        if binding.end() > u64::from(MAX_REGISTER_INDEX) + 1 {
            self.error(
                "T022",
                annotation.span,
                "resource register range is too large",
            );
            return;
        }
        self.resource_bindings
            .insert(span_key(variable.span), binding);
    }

    /// One warning per intersecting range, without expanding resource arrays or
    /// enumerating all pairs. Distinct classes and spaces have separate ranges.
    pub(crate) fn check_binding_overlaps(&mut self, module: &Module) {
        let mut ranges: Vec<_> = module
            .declarations
            .iter()
            .filter_map(|declaration| {
                let Declaration::Global(variable) = declaration else {
                    return None;
                };
                self.resource_bindings
                    .get(&span_key(variable.span))
                    .map(|&binding| (binding, variable))
            })
            .collect();
        ranges.sort_by_key(|(b, _)| (b.register.class, b.register.space, b.register.index));
        let mut previous: Option<(ResourceBinding, &Variable)> = None;
        for (binding, variable) in ranges {
            if let Some((prior, declaration)) = previous
                && prior.register.class == binding.register.class
                && prior.register.space == binding.register.space
                && u64::from(binding.register.index) < prior.end()
            {
                let name = self.source_name(&variable.name);
                let other = self.source_name(&declaration.name);
                self.diagnostics.push(
                    Diagnostic::warning(
                        "T023",
                        variable.register.unwrap().span,
                        format!(
                            "explicit binding for resource '{name}' overlaps resource '{other}'"
                        ),
                    )
                    .note(format!(
                        "overlapping resource uses {} with {} register(s)",
                        prior.register, prior.count
                    )),
                );
                if prior.end() >= binding.end() {
                    continue;
                }
            }
            previous = Some((binding, variable));
        }
    }

    pub(crate) fn allocate_resource_bindings(&mut self, module: &Module) {
        if self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            return;
        }
        let buffer_members: std::collections::BTreeSet<_> = module
            .constant_buffers
            .iter()
            .flat_map(|b| &b.members)
            .map(|m| span_key(m.span))
            .collect();
        let mut globals: Vec<_> = module
            .declarations
            .iter()
            .filter_map(|declaration| {
                let Declaration::Global(variable) = declaration else {
                    return None;
                };
                if buffer_members.contains(&span_key(variable.span)) {
                    return None;
                }
                (!variable
                    .modifiers
                    .iter()
                    .any(|m| matches!(m.as_str(), "static" | "groupshared")))
                .then_some(variable)
            })
            .collect();
        // Linking emits dependencies first. Parameter layout instead starts
        // with the root module, then follows module discovery order.
        globals.sort_by_key(|v| self.symbols.get(&v.name).map_or(0, |o| o.module.0));
        let mut allocator = RegisterAllocator::default();
        for binding in self.resource_bindings.values() {
            // Every explicit range has already passed the same range bounds.
            if let Err(error) = allocator.reserve(*binding) {
                self.error(
                    "T022",
                    module.declarations.first().map_or_default(declaration_span),
                    error.to_string(),
                );
                return;
            }
        }
        if let Some(uniform) = globals
            .iter()
            .find(|v| !self.declaration_types[&span_key(v.span)].is_resource())
        {
            // Preserve the shared buffer's slot for IR, emission, and reflection.
            match allocator.allocate(RegisterClass::ConstantBuffer, 1, 0) {
                Ok(binding) => self.global_uniform_binding = Some(binding),
                Err(error) => {
                    self.error("T022", uniform.span, error.to_string());
                    return;
                }
            }
        }
        for global in globals {
            let key = span_key(global.span);
            if self.is_empty_constant_buffer(&self.declaration_types[&key]) {
                continue;
            }
            if self.resource_bindings.contains_key(&key) {
                continue;
            }
            let range = match resource_range(&self.declaration_types[&key]) {
                Ok(Some(range)) => range,
                Ok(None) => continue,
                Err(error) => {
                    self.error("T022", global.span, error.to_string());
                    continue;
                }
            };
            match allocator.allocate(range.0, range.1, 0) {
                Ok(binding) => {
                    self.resource_bindings.insert(key, binding);
                }
                Err(error) => self.error("T022", global.span, error.to_string()),
            }
        }
    }

    /// Empty value types do not consume a constant-buffer register.
    fn is_empty_constant_buffer(&self, mut ty: &Type) -> bool {
        while let Type::Array(element, _) = ty {
            ty = element;
        }
        let Type::ConstantBuffer(element) = ty else {
            return false;
        };
        let mut stack = vec![element.as_ref()];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(ty) = stack.pop() {
            match ty {
                Type::Array(element, _) => stack.push(element),
                Type::Struct(name) => {
                    if visited.insert(name) {
                        let Some(fields) = self.structures.get(name) else {
                            return false;
                        };
                        stack.extend(fields.values());
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

fn resource_range(mut ty: &Type) -> Result<Option<(RegisterClass, u32)>, AllocationError> {
    let mut dimensions = vec![];
    while let Type::Array(element, size) = ty {
        dimensions.push(*size);
        ty = element;
    }
    let class = match ty {
        Type::ConstantBuffer(_) => RegisterClass::ConstantBuffer,
        Type::ByteBuffer { writable }
        | Type::Buffer { writable, .. }
        | Type::Texture { writable, .. } => {
            if *writable {
                RegisterClass::UnorderedAccess
            } else {
                RegisterClass::ShaderResource
            }
        }
        Type::Sampler { .. } => RegisterClass::Sampler,
        _ => return Ok(None),
    };
    let count = dimensions.into_iter().try_fold(1_u32, |count, size| {
        count.checked_mul(size).ok_or(AllocationError::InvalidRange)
    })?;
    Ok(Some((class, count)))
}

fn declaration_span(declaration: &Declaration) -> slang_source::Span {
    match declaration {
        Declaration::Global(v) => v.span,
        Declaration::Struct(s) => s.span,
        Declaration::Function(f) => f.span,
        Declaration::TypeAlias(a) => a.span,
    }
}
