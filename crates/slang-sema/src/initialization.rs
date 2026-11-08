use crate::{Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Expr, ExprId};

/// A checked value construction, with every omitted field made explicit.
#[derive(Clone, Debug)]
pub struct Initialization {
    pub ty: Type,
    pub kind: InitializationKind,
}

#[derive(Clone, Debug)]
pub enum InitializationKind {
    Value(Box<Expr>),
    /// Evaluate a default in its declaration scope, outside caller bindings.
    DeclarationDefault(Box<Expr>),
    Zero,
    Aggregate(Vec<Initialization>),
}

impl Checker {
    pub(crate) fn struct_constructor(
        &mut self,
        name: &str,
        arguments: &[Expr],
        types: &[Type],
        callee: ExprId,
        span: Span,
    ) -> Type {
        let target = Type::Struct(name.into());
        if types == [target.clone()] {
            self.initializations.insert(
                callee,
                Initialization {
                    ty: target.clone(),
                    kind: InitializationKind::Value(Box::new(arguments[0].clone())),
                },
            );
            return target;
        }
        if !self.constant_buffer_data(&target, &mut std::collections::BTreeSet::new()) {
            self.error(
                "T020",
                span,
                "memberwise resource-valued struct constructors are not implemented",
            );
            return Type::Error;
        }
        let structure = self.struct_definitions[name].clone();
        let public = self.public_types.contains(name);
        let eligible = |field: &slang_syntax::Variable| {
            !public || field.modifiers.iter().any(|m| m == "public")
        };
        let count = structure.fields.iter().filter(|f| eligible(f)).count();
        if arguments.len() > count {
            self.error(
                "T014",
                span,
                format!(
                    "too many constructor arguments for '{}': expected at most {count}",
                    self.source_name(name)
                ),
            );
            return Type::Error;
        }
        let mut budget = 65536;
        let mut supplied = 0;
        let mut fields = vec![];
        for field in &structure.fields {
            let ty = self.structures[name][&field.name].clone();
            let value = if eligible(field) && supplied < arguments.len() {
                self.coerce_value(&arguments[supplied], &ty);
                let expr = arguments[supplied].clone();
                supplied += 1;
                Initialization {
                    ty,
                    kind: InitializationKind::Value(Box::new(expr)),
                }
            } else {
                self.field_default(field, &ty, 0, &mut budget)
            };
            fields.push(value);
        }
        if fields
            .iter()
            .any(|f| f.ty.is_resource() && !self.is_empty_type(&f.ty))
        {
            self.error(
                "T020",
                span,
                "memberwise resource-valued struct constructors are not implemented",
            );
            return Type::Error;
        }
        self.initializations.insert(
            callee,
            Initialization {
                ty: target.clone(),
                kind: InitializationKind::Aggregate(fields),
            },
        );
        target
    }

    fn field_default(
        &mut self,
        field: &slang_syntax::Variable,
        ty: &Type,
        depth: usize,
        budget: &mut usize,
    ) -> Initialization {
        if let Some(value) = &field.initializer {
            Initialization {
                ty: ty.clone(),
                kind: InitializationKind::DeclarationDefault(Box::new(value.clone())),
            }
        } else {
            self.default_initialization(ty, field.span, depth, budget)
        }
    }

    pub(crate) fn default_initialization(
        &mut self,
        ty: &Type,
        span: Span,
        depth: usize,
        budget: &mut usize,
    ) -> Initialization {
        if depth >= 32 || *budget == 0 {
            self.error("T020", span, "aggregate initialization limit exceeded");
            return Initialization {
                ty: Type::Error,
                kind: InitializationKind::Zero,
            };
        }
        *budget -= 1;
        let kind = match ty {
            Type::Struct(name) => {
                let fields = self.struct_definitions[name].fields.clone();
                let mut values = vec![];
                for field in fields {
                    let element = self.structures[name][&field.name].clone();
                    values.push(self.field_default(&field, &element, depth + 1, budget));
                }
                InitializationKind::Aggregate(values)
            }
            Type::Array(element, length) => {
                if usize::try_from(*length).unwrap_or(usize::MAX) > *budget {
                    self.error("T020", span, "aggregate initialization size limit exceeded");
                    return Initialization {
                        ty: Type::Error,
                        kind: InitializationKind::Zero,
                    };
                }
                let mut values = vec![];
                for _ in 0..*length {
                    values.push(self.default_initialization(element, span, depth + 1, budget));
                }
                InitializationKind::Aggregate(values)
            }
            Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..) | Type::Error => {
                InitializationKind::Zero
            }
            _ => {
                self.error(
                    "T020",
                    span,
                    "default initialization of resource-valued fields is not implemented",
                );
                InitializationKind::Zero
            }
        };
        Initialization {
            ty: ty.clone(),
            kind,
        }
    }
}
