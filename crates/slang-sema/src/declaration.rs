use crate::{
    Type,
    checker::{Binding, Checker, Signature, Storage, span_key},
};
use slang_syntax::{Declaration, Module, TypeArgument, TypeRef, Variable};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(crate) enum VariableScope {
    Global,
    Local,
    Parameter,
}

impl Checker {
    pub fn resolve_type(&mut self, reference: &TypeRef) -> Type {
        let mut ty = if matches!(reference.name.as_str(), "vector" | "matrix") {
            self.numeric_type_application(reference)
        } else if reference.name == "ConstantBuffer" {
            match reference.arguments.as_slice() {
                [element] => {
                    let matrix_major = match element {
                        TypeArgument::Type(element) => Some(element),
                        _ => None,
                    };
                    let element = self.type_argument(element);
                    if element.is_resource() && !self.is_empty_type(&element) {
                        self.error(
                            "T002",
                            reference.span,
                            "constant buffers require a data element type",
                        );
                    }
                    Type::ConstantBuffer {
                        element: Box::new(element),
                        matrix_major: matrix_major
                            .and_then(|r| self.reference_matrix_major(r))
                            .filter(|m| *m != self.default_matrix_major),
                    }
                }
                _ => {
                    self.error(
                        "T002",
                        reference.span,
                        "constant buffers require one element type",
                    );
                    Type::Error
                }
            }
        } else if matches!(
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
                let element = self.type_argument(&reference.arguments[0]);
                if element == Type::Void
                    || element.is_resource()
                    || matches!(element, Type::Array(..))
                {
                    self.error("T002", reference.span, "invalid buffer element type");
                }
                Type::Buffer {
                    element: Box::new(element),
                    writable: reference.name.starts_with("RW"),
                    matrix_major: match &reference.arguments[0] {
                        TypeArgument::Type(element) => self
                            .reference_matrix_major(element)
                            .filter(|major| *major != self.default_matrix_major),
                        _ => None,
                    },
                }
            }
        } else if Type::texture(&reference.name, Type::Error).is_some() {
            let element = match reference.arguments.as_slice() {
                [] => Type::Vector(crate::Scalar::Float, 4),
                [element] => self.type_argument(element),
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
        } else if self.aliases.contains_key(&reference.name)
            && self.module_symbol_visible(&reference.name)
        {
            self.resolve_alias(&reference.name, reference.span)
        } else if self.structures.contains_key(&reference.name)
            && self.module_symbol_visible(&reference.name)
        {
            Type::Struct(reference.name.clone())
        } else {
            self.error(
                "T002",
                reference.span,
                format!("unknown or unsupported type '{}'", reference.name),
            );
            Type::Error
        };
        for size in reference.arrays.iter().rev() {
            if ty == Type::Void {
                self.error("T002", reference.span, "invalid array element type");
            }
            let mut element = &ty;
            let mut depth = 0;
            while let Type::Array(inner, _) = element {
                depth += 1;
                element = inner;
            }
            if depth >= 32 {
                self.error("T002", reference.span, "array type nesting limit exceeded");
                return Type::Error;
            }
            let Some(size) = self
                .numeric_shape_value(size)
                .and_then(|value| u32::try_from(value).ok())
            else {
                self.error(
                    "T002",
                    size.span,
                    "array size must be a nonnegative constant integer",
                );
                return Type::Error;
            };
            ty = Type::Array(Box::new(ty), size);
        }
        if reference.matrix_major.is_some() {
            self.check_matrix_type(&ty, reference.span);
        }
        ty
    }

    fn type_argument(&mut self, argument: &TypeArgument) -> Type {
        match argument {
            TypeArgument::Type(reference) => self.resolve_type(reference),
            TypeArgument::Integer { span, .. } => {
                self.error("T002", *span, "expected a type argument, found an integer");
                Type::Error
            }
            TypeArgument::Expression(expression) => {
                self.error(
                    "T002",
                    expression.span,
                    "expected a type argument, found a value",
                );
                Type::Error
            }
        }
    }

    fn numeric_type_application(&mut self, reference: &TypeRef) -> Type {
        let expected = if reference.name == "vector" { 2 } else { 3 };
        if reference.arguments.len() > expected {
            self.error(
                "T002",
                reference.span,
                format!("{} accepts at most {expected} arguments", reference.name),
            );
            return Type::Error;
        }
        let component = reference
            .arguments
            .first()
            .map_or(Type::Scalar(crate::Scalar::Float), |argument| {
                self.type_argument(argument)
            });
        let Type::Scalar(scalar) = component else {
            if component != Type::Error {
                self.error(
                    "T002",
                    reference.span,
                    "vector and matrix components require a scalar type",
                );
            }
            return Type::Error;
        };
        let mut dimensions = vec![];
        for argument in reference.arguments.iter().skip(1) {
            let (value, span) = match argument {
                TypeArgument::Integer { value, span } => (i64::from(*value), *span),
                TypeArgument::Expression(expression) => {
                    let Some(value) = self.numeric_shape_value(expression) else {
                        self.error(
                            "T002",
                            expression.span,
                            "numeric shape arguments require scalar integer constants",
                        );
                        return Type::Error;
                    };
                    (value, expression.span)
                }
                TypeArgument::Type(_) => {
                    self.error(
                        "T002",
                        reference.span,
                        "numeric shape arguments require scalar integer constants",
                    );
                    return Type::Error;
                }
            };
            if !(1..=4).contains(&value) {
                self.error(
                    "T002",
                    span,
                    "numeric shape dimensions require values from 1 through 4",
                );
                return Type::Error;
            }
            dimensions.push(value as u8);
        }
        dimensions.resize(expected - 1, 4);
        if reference.name == "vector" {
            Type::Vector(scalar, dimensions[0])
        } else {
            Type::Matrix(scalar, dimensions[0], dimensions[1])
        }
    }

    pub(crate) fn declare(&mut self, module: &Module) {
        let names = self.declare_types(module);
        for declaration in &module.declarations {
            let name = match declaration {
                Declaration::TypeAlias(a) => &a.name,
                Declaration::Struct(s) => &s.name,
                Declaration::Global(v) => &v.name,
                Declaration::Function(f) => &f.name,
            };
            self.current_module = self
                .symbols
                .get(name)
                .map_or(slang_module::ModuleId(0), |o| o.module);
            match declaration {
                Declaration::TypeAlias(_) => {}
                Declaration::Struct(structure) => {
                    self.check_visibility_modifiers(&structure.modifiers, structure.span);
                    let mut fields = BTreeMap::new();
                    for field in &structure.fields {
                        let ty = self.resolve_type(&field.ty);
                        if field.modifiers.iter().any(|m| m == "public") {
                            self.public_fields
                                .insert((structure.name.clone(), field.name.clone()));
                            if !self.public_types.contains(&structure.name) {
                                self.error(
                                    "T018",
                                    field.span,
                                    if module
                                        .constant_buffers
                                        .iter()
                                        .any(|b| b.element_type == structure.name)
                                    {
                                        "a public cbuffer field requires a public cbuffer"
                                    } else {
                                        "a public field requires a public struct"
                                    },
                                );
                            }
                            self.require_public_type(&ty, field.span);
                        }
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
                    if let Some(owner) = &function.constructor {
                        self.constructors
                            .insert(owner.clone(), function.name.clone());
                    }
                    if let Some(method) = &function.method {
                        self.methods.insert(
                            (method.owner.clone(), method.name.clone()),
                            function.name.clone(),
                        );
                        if self
                            .structures
                            .get(&method.owner)
                            .is_some_and(|fields| fields.contains_key(&method.name))
                        {
                            self.error("T003", function.span, "method name conflicts with a field");
                        }
                    }
                    if self.scopes[0].contains_key(&function.name)
                        || names.contains(&function.name)
                        || Type::builtin(self.source_name(&function.name)).is_some()
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
                    if function.modifiers.iter().any(|m| m == "public") {
                        self.require_public_type(&result, function.span);
                        for (ty, parameter) in parameters.iter().zip(&function.parameters) {
                            self.require_public_type(ty, parameter.span);
                        }
                    }
                    let output_parameters = function
                        .parameters
                        .iter()
                        .map(|p| p.modifiers.iter().any(|m| m == "out" || m == "inout"))
                        .collect();
                    let overloads = self.functions.entry(function.name.clone()).or_default();
                    if function.constructor.is_none()
                        && let Some(existing) = overloads.iter_mut().find(|s| {
                            s.parameters == parameters
                                && s.method.as_ref().map(|m| m.is_static)
                                    == function.method.as_ref().map(|m| m.is_static)
                        })
                    {
                        let duplicate = existing.result != result
                            || existing.output_parameters != output_parameters
                            || existing.public != function.modifiers.iter().any(|m| m == "public")
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
                            declaration: span_key(function.span),
                            parameters,
                            output_parameters,
                            result,
                            defined: function.body.is_some(),
                            public: function.modifiers.iter().any(|m| m == "public"),
                            target_name: function.name.clone(),
                            defaults: function
                                .parameters
                                .iter()
                                .map(|p| p.initializer.clone())
                                .collect(),
                            method: function.method.clone(),
                        });
                    }
                }
            }
        }
        // Preserve selected overloads when module visibility, one-component
        // shapes, half/float types, or integer literal ranking differ in HLSL.
        let mut target_names = declaration_names(module);
        target_names.extend(self.symbols.keys().cloned());
        let selected_names: BTreeSet<_> = self
            .constructors
            .values()
            .cloned()
            .chain(module.declarations.iter().filter_map(|d| {
                if let Declaration::Function(f) = d
                    && (f.method.is_some() || f.attributes.iter().any(|a| a.name == "ForceInline"))
                {
                    Some(f.name.clone())
                } else {
                    None
                }
            }))
            .collect();
        for (name, overloads) in &mut self.functions {
            if overloads.len() > 1
                && (selected_names.contains(name)
                    || self.symbols.get(name).is_some_and(|o| o.module.0 != 0)
                    || overloads
                        .iter()
                        .any(|signature| signature.parameters.iter().any(collapsed_hlsl_type)))
            {
                for (index, signature) in overloads.iter_mut().enumerate() {
                    let mut target = format!("{name}__overload_{index}");
                    while !target_names.insert(target.clone()) {
                        target.push('_');
                    }
                    signature.target_name = target;
                }
            }
        }
        for declaration in &module.declarations {
            if let Declaration::Function(function) = declaration {
                let parameters: Vec<_> = function
                    .parameters
                    .iter()
                    .map(|p| self.resolve_type(&p.ty))
                    .collect();
                if let Some(signature) = self.functions.get(&function.name).and_then(|overloads| {
                    overloads.iter().find(|s| {
                        s.parameters == parameters
                            && s.method.as_ref().map(|m| m.is_static)
                                == function.method.as_ref().map(|m| m.is_static)
                            && (function.constructor.is_none()
                                || s.declaration == span_key(function.span))
                    })
                }) {
                    self.function_names
                        .insert(span_key(function.span), signature.target_name.clone());
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
        self.validate_value_types();
    }

    pub(crate) fn bind(&mut self, variable: &Variable, context: VariableScope) {
        if matches!(context, VariableScope::Global)
            && variable.initializer.is_some()
            && variable.modifiers.iter().any(|m| m == "const")
            && !variable.modifiers.iter().any(|m| m == "static")
        {
            self.error(
                "T004",
                variable.span,
                "global const with an initializer requires static",
            );
        }
        let source_name = self.source_name(&variable.name);
        if Type::builtin(source_name).is_some()
            || self
                .source_types
                .contains(&(self.current_module, source_name.to_owned()))
        {
            self.error("T003", variable.span, "variable name conflicts with a type");
        }
        let ty = self.resolve_type(&variable.ty);
        self.check_matrix_layout(&ty, &variable.ty, &variable.modifiers, variable.span);
        if let Some(pack) = variable.pack_offset
            && (!matches!(context, VariableScope::Global)
                || !variable.modifiers.iter().any(|m| m == "static"))
        {
            self.error(
                "T022",
                pack.span,
                "packoffset currently requires a direct cbuffer field",
            );
        }
        self.check_register(variable, &ty, context);
        if matches!(context, VariableScope::Global)
            && variable.modifiers.iter().any(|m| m == "public")
        {
            self.require_public_type(&ty, variable.span);
        }
        if ty == Type::Void {
            self.error("T002", variable.span, "a variable cannot have type void");
        }
        for modifier in &variable.modifiers {
            let valid = match context {
                VariableScope::Global => matches!(
                    modifier.as_str(),
                    "const"
                        | "static"
                        | "uniform"
                        | "groupshared"
                        | "public"
                        | "internal"
                        | "row_major"
                        | "column_major"
                ),
                VariableScope::Local => matches!(
                    modifier.as_str(),
                    "const" | "static" | "row_major" | "column_major"
                ),
                VariableScope::Parameter => {
                    matches!(
                        modifier.as_str(),
                        "const"
                            | "in"
                            | "out"
                            | "inout"
                            | "nointerpolation"
                            | "row_major"
                            | "column_major"
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
        self.check_visibility_modifiers(&variable.modifiers, variable.span);
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
        let constant = variable
            .initializer
            .as_ref()
            .filter(|_| {
                matches!(
                    ty,
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
            })
            .filter(|_| variable.modifiers.iter().any(|m| m == "const"))
            .filter(|_| !matches!(context, VariableScope::Parameter))
            .and_then(|expr| self.integer_constant(expr));
        let shape_constant = if matches!(context, VariableScope::Local)
            && variable.modifiers.iter().any(|m| m == "const")
            && matches!(
                ty,
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
            ) {
            variable
                .initializer
                .as_ref()
                .and_then(|expr| self.numeric_shape_value(expr))
                .map(|v| {
                    if variable
                        .initializer
                        .as_ref()
                        .and_then(|expr| self.expression_types.get(&expr.id))
                        != Some(&ty)
                        || ty == Type::Scalar(crate::Scalar::Bool)
                    {
                        crate::constant::normalize(v, &ty)
                    } else {
                        v
                    }
                })
        } else {
            None
        };
        self.declaration_types
            .insert(span_key(variable.span), ty.clone());
        let scope = self.scopes.last_mut().unwrap();
        if scope
            .insert(
                variable.name.clone(),
                Binding {
                    ty,
                    writable,
                    constant,
                    shape_constant,
                    storage: match context {
                        VariableScope::Global
                            if variable.modifiers.iter().any(|m| m == "groupshared") =>
                        {
                            Storage::GroupShared(span_key(variable.span))
                        }
                        VariableScope::Global => Storage::Global(span_key(variable.span)),
                        VariableScope::Local | VariableScope::Parameter => Storage::Local,
                    },
                },
            )
            .is_some()
        {
            self.error(
                "T003",
                variable.span,
                format!("duplicate variable '{}'", self.source_name(&variable.name)),
            );
        }
    }
}

fn collapsed_hlsl_type(ty: &Type) -> bool {
    match ty {
        Type::Scalar(
            crate::Scalar::Half
            | crate::Scalar::Int16
            | crate::Scalar::Uint16
            | crate::Scalar::Int64
            | crate::Scalar::Uint64
            | crate::Scalar::IntPtr
            | crate::Scalar::UintPtr,
        )
        | Type::Vector(
            crate::Scalar::Half
            | crate::Scalar::Int16
            | crate::Scalar::Uint16
            | crate::Scalar::Int64
            | crate::Scalar::Uint64
            | crate::Scalar::IntPtr
            | crate::Scalar::UintPtr,
            _,
        )
        | Type::Matrix(
            crate::Scalar::Half
            | crate::Scalar::Int16
            | crate::Scalar::Uint16
            | crate::Scalar::Int64
            | crate::Scalar::Uint64
            | crate::Scalar::IntPtr
            | crate::Scalar::UintPtr,
            _,
            _,
        ) => true,
        Type::Vector(_, 1) | Type::Matrix(_, 1, _) | Type::Matrix(_, _, 1) => true,
        Type::Array(element, _)
        | Type::Buffer { element, .. }
        | Type::Texture { element, .. }
        | Type::ConstantBuffer { element, .. } => collapsed_hlsl_type(element),
        _ => false,
    }
}

/// Reserve lexical declarations as well as globals before generating function names.
fn declaration_names(module: &Module) -> BTreeSet<String> {
    use slang_syntax::StmtKind;
    let mut names = BTreeSet::new();
    let mut pending = vec![];
    for declaration in &module.declarations {
        match declaration {
            Declaration::Function(function) => {
                names.insert(function.name.clone());
                names.extend(function.parameters.iter().map(|p| p.name.clone()));
                pending.extend(function.body.as_ref());
            }
            Declaration::Global(variable) => {
                names.insert(variable.name.clone());
            }
            Declaration::Struct(structure) => {
                names.insert(structure.name.clone());
            }
            Declaration::TypeAlias(alias) => {
                names.insert(alias.name.clone());
            }
        }
    }
    while let Some(statement) = pending.pop() {
        match &statement.kind {
            StmtKind::Block(statements) => pending.extend(statements),
            StmtKind::Variable(variable) => {
                names.insert(variable.name.clone());
            }
            StmtKind::Variables(variables) => {
                names.extend(variables.iter().map(|v| v.name.clone()))
            }
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                pending.push(then_branch.as_ref());
                pending.extend(else_branch.as_deref());
            }
            StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => {
                pending.push(body.as_ref())
            }
            StmtKind::For {
                initializer, body, ..
            } => pending.extend([initializer.as_ref(), body.as_ref()]),
            StmtKind::Switch { cases, .. } => {
                for case in cases {
                    pending.extend(&case.statements);
                }
            }
            _ => {}
        }
    }
    names
}
