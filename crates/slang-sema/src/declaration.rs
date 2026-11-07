use crate::{
    Type,
    checker::{Binding, Checker, Signature, span_key},
};
use slang_source::Span;
use slang_syntax::{Declaration, Module, TypeRef, Variable};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(crate) enum VariableScope {
    Global,
    Local,
    Parameter,
}

impl Checker {
    pub fn resolve_type(&mut self, reference: &TypeRef) -> Type {
        let mut ty = if matches!(
            reference.name.as_str(),
            "StructuredBuffer" | "RWStructuredBuffer"
        ) {
            if reference.arguments.len() != 1 {
                self.error(
                    "T002",
                    reference.span,
                    "structured buffers require one element type",
                );
                Type::Error
            } else {
                let element = self.resolve_type(&reference.arguments[0]);
                if element == Type::Void || element.is_resource() {
                    self.error("T002", reference.span, "invalid buffer element type");
                }
                Type::Buffer {
                    element: Box::new(element),
                    writable: reference.name.starts_with("RW"),
                }
            }
        } else if Type::texture(&reference.name, Type::Error).is_some() {
            let element = match reference.arguments.as_slice() {
                [] => Type::Vector(crate::Scalar::Float, 4),
                [element] => self.resolve_type(element),
                _ => {
                    self.error(
                        "T002",
                        reference.span,
                        "textures accept at most one element type",
                    );
                    Type::Error
                }
            };
            if element != Type::Error
                && !matches!(
                    element,
                    Type::Scalar(
                        crate::Scalar::Half
                            | crate::Scalar::Float
                            | crate::Scalar::Int
                            | crate::Scalar::Uint
                    ) | Type::Vector(
                        crate::Scalar::Half
                            | crate::Scalar::Float
                            | crate::Scalar::Int
                            | crate::Scalar::Uint,
                        _
                    )
                )
            {
                self.error(
                    "T002",
                    reference.span,
                    "texture elements require half, float, int, or uint scalar/vector types",
                );
            }
            Type::texture(&reference.name, element).unwrap()
        } else if !reference.arguments.is_empty() {
            self.error(
                "T002",
                reference.span,
                "user-defined generic types are not implemented",
            );
            Type::Error
        } else if let Some(ty) = Type::builtin(&reference.name) {
            ty
        } else if self.structures.contains_key(&reference.name) {
            Type::Struct(reference.name.clone())
        } else {
            self.error(
                "T002",
                reference.span,
                format!("unknown or unsupported type '{}'", reference.name),
            );
            Type::Error
        };
        if let Some(size) = reference.array {
            if ty == Type::Void || ty.is_resource() {
                self.error("T002", reference.span, "invalid array element type");
            }
            ty = Type::Array(Box::new(ty), size);
        }
        ty
    }

    pub(crate) fn declare(&mut self, module: &Module) {
        let mut names = BTreeSet::new();
        for declaration in &module.declarations {
            if let Declaration::Struct(structure) = declaration {
                if Type::builtin(&structure.name).is_some() || !names.insert(structure.name.clone())
                {
                    self.error(
                        "T003",
                        structure.span,
                        format!("duplicate type '{}'", structure.name),
                    );
                }
                self.structures
                    .insert(structure.name.clone(), BTreeMap::new());
            }
        }
        for declaration in &module.declarations {
            match declaration {
                Declaration::Struct(structure) => {
                    let mut fields = BTreeMap::new();
                    for field in &structure.fields {
                        let ty = self.resolve_type(&field.ty);
                        if ty == Type::Void {
                            self.error("T002", field.span, "a field cannot have type void");
                        }
                        if fields.insert(field.name.clone(), ty.clone()).is_some() {
                            self.error("T003", field.span, "duplicate struct field");
                        }
                        self.declaration_types.insert(span_key(field.span), ty);
                    }
                    self.structures.insert(structure.name.clone(), fields);
                }
                Declaration::Global(variable) => {
                    if names.contains(&variable.name) {
                        self.error("T003", variable.span, "value name conflicts with a type");
                    }
                    self.bind(variable, VariableScope::Global);
                }
                Declaration::Function(function) => {
                    if self.scopes[0].contains_key(&function.name)
                        || names.contains(&function.name)
                        || Type::builtin(&function.name).is_some()
                    {
                        self.error(
                            "T003",
                            function.span,
                            "function name conflicts with another declaration",
                        );
                    }
                    let result = self.resolve_type(&function.return_type);
                    self.declaration_types
                        .insert(span_key(function.span), result.clone());
                    let parameters: Vec<Type> = function
                        .parameters
                        .iter()
                        .map(|p| self.resolve_type(&p.ty))
                        .collect();
                    let output_parameters = function
                        .parameters
                        .iter()
                        .map(|p| p.modifiers.iter().any(|m| m == "out" || m == "inout"))
                        .collect();
                    let overloads = self.functions.entry(function.name.clone()).or_default();
                    if let Some(existing) =
                        overloads.iter_mut().find(|s| s.parameters == parameters)
                    {
                        let duplicate = existing.result != result
                            || existing.output_parameters != output_parameters
                            || existing.defined && function.body.is_some();
                        existing.defined |= function.body.is_some();
                        if duplicate {
                            self.error(
                                "T003",
                                function.span,
                                "duplicate or conflicting function signature",
                            );
                        }
                    } else {
                        overloads.push(Signature {
                            parameters,
                            output_parameters,
                            result,
                            defined: function.body.is_some(),
                        });
                    }
                }
            }
        }
        for declaration in &module.declarations {
            if let Declaration::Global(variable) = declaration
                && self.functions.contains_key(&variable.name)
            {
                self.error(
                    "T003",
                    variable.span,
                    "global name conflicts with a function",
                );
            }
        }
        for name in self.structures.keys().cloned().collect::<Vec<_>>() {
            if self.recursive_struct(&name, &mut BTreeSet::new()) {
                self.error(
                    "T002",
                    Span::default(),
                    format!("recursive or excessively nested value type '{name}'"),
                );
            }
        }
    }

    fn recursive_struct(&self, name: &str, path: &mut BTreeSet<String>) -> bool {
        if path.len() >= 128 {
            return true;
        }
        if !path.insert(name.into()) {
            return true;
        }
        let recursive = self.structures.get(name).is_some_and(|fields| {
            fields.values().any(|ty| {
                let mut ty = ty;
                while let Type::Array(element, _) = ty {
                    ty = element;
                }
                matches!(ty, Type::Struct(child) if self.recursive_struct(child, path))
            })
        });
        path.remove(name);
        recursive
    }

    pub(crate) fn bind(&mut self, variable: &Variable, context: VariableScope) {
        if Type::builtin(&variable.name).is_some() || self.structures.contains_key(&variable.name) {
            self.error("T003", variable.span, "variable name conflicts with a type");
        }
        let ty = self.resolve_type(&variable.ty);
        if ty == Type::Void {
            self.error("T002", variable.span, "a variable cannot have type void");
        }
        for modifier in &variable.modifiers {
            let valid = match context {
                VariableScope::Global => matches!(
                    modifier.as_str(),
                    "const" | "static" | "uniform" | "groupshared"
                ),
                VariableScope::Local => matches!(modifier.as_str(), "const" | "static"),
                VariableScope::Parameter => {
                    matches!(
                        modifier.as_str(),
                        "const" | "in" | "out" | "inout" | "nointerpolation"
                    )
                }
            };
            if !valid {
                self.error(
                    "T001",
                    variable.span,
                    format!("modifier '{modifier}' is unsupported in this declaration context"),
                );
            }
        }
        self.check_interpolation(&variable.modifiers, variable.span);
        let directions = variable
            .modifiers
            .iter()
            .filter(|m| matches!(m.as_str(), "in" | "out" | "inout"))
            .count();
        if directions > 1
            || variable.modifiers.iter().any(|m| m == "const")
                && variable
                    .modifiers
                    .iter()
                    .any(|m| m == "out" || m == "inout")
        {
            self.error("T001", variable.span, "conflicting parameter modifiers");
        }
        let global_mutable = !matches!(context, VariableScope::Global)
            || variable
                .modifiers
                .iter()
                .any(|m| m == "static" || m == "groupshared");
        let writable = global_mutable
            && !variable
                .modifiers
                .iter()
                .any(|m| m == "const" || m == "uniform");
        self.declaration_types
            .insert(span_key(variable.span), ty.clone());
        let scope = self.scopes.last_mut().unwrap();
        if scope
            .insert(variable.name.clone(), Binding { ty, writable })
            .is_some()
        {
            self.error(
                "T003",
                variable.span,
                format!("duplicate variable '{}'", variable.name),
            );
        }
    }
}
