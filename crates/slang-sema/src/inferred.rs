//! Infer variable types after declaration headers and initializer dependencies are available.

use crate::{
    Type,
    checker::{Binding, Checker, Storage, span_key},
};
use slang_syntax::{Declaration, Expr, ExprKind, Module, TypeRef, Variable};
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn variable_type(&mut self, variable: &Variable) -> Type {
        if !variable.ty.is_inferred() {
            return self.resolve_type(&variable.ty);
        }
        if let Some(ty) = self.declaration_types.get(&span_key(variable.span))
            && *ty != Type::Error
        {
            return ty.clone();
        }
        let Some(initializer) = &variable.initializer else {
            self.error("T034", variable.span, "an inferred variable requires an initializer");
            return Type::Error;
        };
        let ty = self.expression(initializer);
        if matches!(ty, Type::InitializerList | Type::Meta(_) | Type::Callable { .. }) {
            self.error(
                "T034",
                initializer.span,
                "initializer does not provide a concrete variable type",
            );
            return Type::Error;
        }
        if ty != Type::Error {
            self.declaration_types.insert(span_key(variable.span), ty.clone());
            if let Some(major) = self.expression_matrix_major(initializer) {
                self.matrix_layouts.insert(span_key(variable.span), major);
            }
        }
        ty
    }

    pub(crate) fn infer_module_variables(&mut self, module: &Module) {
        let mut pending = BTreeMap::new();
        for declaration in &module.declarations {
            match declaration {
                Declaration::Global(variable) if variable.ty.is_inferred() => {
                    pending.entry(span_key(variable.span)).or_insert((None, variable));
                }
                Declaration::Struct(owner) => {
                    for variable in &owner.fields {
                        if variable.ty.is_inferred() {
                            // Direct cbuffer members share their source key with
                            // a global whose initializer has been normalized away.
                            pending.insert(
                                span_key(variable.span),
                                (Some(owner.name.as_str()), variable),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        while !pending.is_empty() {
            let mut progress = false;
            pending.retain(|_, (owner, variable)| {
                self.current_static_type = owner.map(str::to_owned).or_else(|| self.static_owners.get(&variable.name).cloned());
                self.current_module = self.symbols.get(owner.unwrap_or(&variable.name)).map_or(Default::default(), |origin| origin.module);
                self.current_type = owner.map(str::to_owned);
                if let Some(owner) = owner {
                    self.scopes.push(BTreeMap::from([("this".into(), Binding {
                        ty: Type::Struct((*owner).into()),
                        matrix_major: None,
                        writable: true,
                        constant: None,
                        shape_constant: None,
                        storage: Storage::Local,
                    })]));
                }
                let count = self.diagnostics.len();
                let ty = self.variable_type(variable);
                if owner.is_some() {
                    self.scopes.pop();
                }
                if ty == Type::Error {
                    return self.diagnostics.len() == count;
                }
                if let Some(owner) = owner {
                    self.structures.get_mut(*owner).unwrap().insert(variable.name.clone(), ty.clone());
                }
                self.check_matrix_layout(&ty, &variable.ty, &variable.modifiers, variable.span);
                for binding in self.scopes[0].values_mut() {
                    if matches!(binding.storage, Storage::Global(key) | Storage::GroupShared(key) if key == span_key(variable.span)) {
                        binding.ty = ty.clone();
                        binding.matrix_major = self.matrix_layouts.get(&span_key(variable.span)).copied();
                    }
                }
                self.require_value_type(&ty, variable.span);
                if variable.modifiers.iter().any(|modifier| modifier == "public") {
                    self.require_public_type(&ty, variable.span);
                }
                if owner.is_none() {
                    self.check_register(variable, &ty, crate::declaration::VariableScope::Global);
                }
                progress = true;
                false
            });
            if !progress {
                if !self.infer_generic_constructors {
                    for (_, variable) in pending.values() {
                        self.error("T034", variable.span, "cannot infer variable type from cyclic or unresolved initializer dependencies");
                    }
                }
                break;
            }
        }
        self.current_type = None;
        self.current_static_type = None;
        self.current_module = Default::default();
    }

    /// Preserve matrix storage qualifiers carried by initializer value types.
    pub(crate) fn expression_matrix_major(
        &mut self,
        expression: &Expr,
    ) -> Option<slang_layout::MatrixMajor> {
        let mut ty = self.expression_types.get(&expression.id)?;
        while let Type::Array(element, _) = ty {
            ty = element;
        }
        if !matches!(ty, Type::Matrix(..)) {
            return None;
        }
        if let Some(key) = self.static_references.get(&expression.id) {
            return self.scopes[0].get(key).and_then(|binding| binding.matrix_major);
        }
        match &expression.kind {
            ExprKind::Name(name) => {
                if let Some(Type::Struct(owner)) =
                    self.implicit_members.get(&expression.id).cloned()
                {
                    self.field_matrix_major(&owner, name)
                } else {
                    self.lookup(name).and_then(|binding| binding.matrix_major)
                }
            }
            ExprKind::Cast {
                target,
                ..
            }
            | ExprKind::TypeApplication(target) => self.reference_matrix_major(target),
            ExprKind::Call {
                callee,
                ..
            } => {
                if matches!(self.expression_types.get(&callee.id), Some(Type::Meta(_))) {
                    match &callee.kind {
                        ExprKind::Name(name) => {
                            let mut reference = TypeRef::inferred(callee.span);
                            reference.name = name.clone();
                            self.reference_matrix_major(&reference)
                        }
                        ExprKind::TypeApplication(reference) => {
                            self.reference_matrix_major(reference)
                        }
                        _ => None,
                    }
                } else {
                    let target = self.call_names.get(&callee.id)?;
                    if target.starts_with("@slang.byte.load.") {
                        return Some(if target.ends_with(".row") {
                            slang_layout::MatrixMajor::Row
                        } else {
                            slang_layout::MatrixMajor::Column
                        });
                    }
                    self.functions
                        .values()
                        .flatten()
                        .find(|signature| &signature.target_name == target)
                        .and_then(|signature| {
                            self.matrix_layouts.get(&signature.declaration).copied()
                        })
                }
            }
            ExprKind::Member {
                base,
                name,
                ..
            } => {
                let Type::Struct(owner) = self.expression_types.get(&base.id)?.value_type() else {
                    return None;
                };
                self.field_matrix_major(&owner.clone(), name)
            }
            ExprKind::Index {
                base,
                ..
            } => self.expression_matrix_major(base),
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let major = self.expression_matrix_major(then_value);
                (major == self.expression_matrix_major(else_value)).then_some(major).flatten()
            }
            ExprKind::Binary {
                op,
                right,
                ..
            } if op == "," => self.expression_matrix_major(right),
            ExprKind::Binary {
                op,
                left,
                right,
            } => {
                let major = self.expression_matrix_major(left);
                if op.ends_with('=')
                    || self
                        .expression_types
                        .get(&left.id)
                        .is_some_and(|ty| matches!(ty.value_type(), Type::Matrix(..)))
                {
                    major
                } else {
                    self.expression_matrix_major(right)
                }
            }
            _ => None,
        }
    }

    fn field_matrix_major(&mut self, owner: &str, name: &str) -> Option<slang_layout::MatrixMajor> {
        let field = self
            .struct_definitions
            .get(owner)?
            .fields
            .iter()
            .find(|field| field.name == name)?
            .clone();
        self.matrix_layouts.get(&span_key(field.span)).copied().or_else(|| {
            field
                .modifiers
                .iter()
                .rev()
                .find_map(|modifier| match modifier.as_str() {
                    "row_major" => Some(slang_layout::MatrixMajor::Row),
                    "column_major" => Some(slang_layout::MatrixMajor::Column),
                    _ => None,
                })
                .or_else(|| self.reference_matrix_major(&field.ty))
        })
    }
}
