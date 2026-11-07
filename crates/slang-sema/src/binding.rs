use crate::{Type, checker::Checker, declaration::VariableScope, span_key};
use slang_layout::{MAX_REGISTER_INDEX, RegisterClass, ResourceBinding};
use slang_source::Diagnostic;
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
        let mut element = ty;
        let mut count = 1_u32;
        while let Type::Array(inner, size) = element {
            let Some(total) = count.checked_mul(*size) else {
                self.error(
                    "T022",
                    annotation.span,
                    "resource register range is too large",
                );
                return;
            };
            count = total;
            element = inner;
        }
        let class = match element {
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
            Type::Error => return,
            _ => {
                self.error(
                    "T022",
                    annotation.span,
                    "register annotations on numeric or aggregate uniforms are not implemented",
                );
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
        let binding = ResourceBinding {
            register: annotation.binding,
            count,
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
}
