use crate::{Initialization, InitializationKind, Type, checker::Checker};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Expr, ExprId, ExprKind, Variable};

impl Checker {
    pub(crate) fn array_constructor(
        &mut self,
        target: &Type,
        arguments: &[Expr],
        callee: ExprId,
        span: Span,
    ) -> Type {
        let plan = if arguments.len() == 1
            && self.expression_types.get(&arguments[0].id) == Some(target)
        {
            Ok(Initialization {
                ty: target.clone(),
                kind: InitializationKind::Value(Box::new(arguments[0].clone())),
            })
        } else if self.constant_buffer_data(target, &mut Default::default()) {
            let mut planner = Planner {
                checker: self,
                remaining: 65536,
            };
            let mut index = 0;
            planner
                .aggregate(arguments, &mut index, target, span, 0)
                .and_then(|plan| {
                    if index == arguments.len() {
                        Ok(plan)
                    } else {
                        Err(Diagnostic::error(
                            "T021",
                            span,
                            format!("too many initializers for {target}"),
                        ))
                    }
                })
        } else {
            Err(Diagnostic::error(
                "T020",
                span,
                "resource-valued array construction is not implemented",
            ))
        };
        match plan {
            Ok(plan) => {
                self.initializations.insert(callee, plan);
                target.clone()
            }
            Err(error) => {
                self.diagnostics.push(error);
                Type::Error
            }
        }
    }

    pub(crate) fn check_value(&mut self, value: &Expr, target: &Type) {
        self.expression(value);
        self.coerce_value(value, target);
    }

    /// Commit contextual typing only after a target or overload has been selected.
    pub(crate) fn coerce_value(&mut self, value: &Expr, target: &Type) {
        if matches!(value.kind, ExprKind::InitializerList(_)) {
            match self.list_plan(value, target) {
                Ok(plan) => {
                    self.initializations.insert(value.id, plan);
                    self.expression_types.insert(value.id, target.clone());
                }
                Err(error) => self.diagnostics.push(error),
            }
        } else {
            let actual = self
                .expression_types
                .get(&value.id)
                .cloned()
                .unwrap_or(Type::Error);
            self.require_conversion(&actual, target, value.span);
        }
    }

    pub(crate) fn can_initialize(&self, value: &Expr, target: &Type) -> bool {
        if matches!(value.kind, ExprKind::InitializerList(_)) {
            self.list_plan(value, target).is_ok()
        } else {
            self.expression_types
                .get(&value.id)
                .is_some_and(|t| t.convertible_to(target))
        }
    }

    fn list_plan(&self, value: &Expr, target: &Type) -> Result<Initialization, Diagnostic> {
        Planner {
            checker: self,
            remaining: 65536,
        }
        .list(value, target, 0)
    }
}

/// Pure planning checks constructor candidates without changing expression types.
struct Planner<'a> {
    checker: &'a Checker,
    remaining: usize,
}

impl Planner<'_> {
    fn charge(&mut self, depth: usize, span: Span) -> Result<(), Diagnostic> {
        if depth >= 32 || self.remaining == 0 {
            return Err(Diagnostic::error(
                "T020",
                span,
                "aggregate initialization limit exceeded",
            ));
        }
        self.remaining -= 1;
        Ok(())
    }

    fn invalid(&self, span: Span, target: &Type) -> Diagnostic {
        Diagnostic::error(
            "T021",
            span,
            format!("invalid initializer list for {target}"),
        )
    }

    fn value(&self, value: &Expr, target: &Type) -> Result<Initialization, Diagnostic> {
        let actual = self
            .checker
            .expression_types
            .get(&value.id)
            .unwrap_or(&Type::Error);
        if !actual.convertible_to(target) {
            return Err(Diagnostic::error(
                "T008",
                value.span,
                format!("cannot convert {actual} to {target}"),
            ));
        }
        Ok(Initialization {
            ty: target.clone(),
            kind: InitializationKind::Value(Box::new(value.clone())),
        })
    }

    fn list(
        &mut self,
        value: &Expr,
        target: &Type,
        depth: usize,
    ) -> Result<Initialization, Diagnostic> {
        self.charge(depth, value.span)?;
        let ExprKind::InitializerList(values) = &value.kind else {
            return self.value(value, target);
        };
        if !self
            .checker
            .constant_buffer_data(target, &mut Default::default())
        {
            return Err(self.invalid(value.span, target));
        }
        if values.is_empty() {
            return self.default(target, value.span, depth + 1);
        }
        if let Type::Struct(name) = target {
            // Synthesized memberwise construction takes precedence over flat lists.
            let structure = &self.checker.struct_definitions[name];
            let public = self.checker.public_types.contains(name);
            let fields: Vec<_> = structure
                .fields
                .iter()
                .filter(|f| eligible(f, public))
                .collect();
            if values.len() <= fields.len()
                && values.iter().zip(&fields).all(|(v, f)| {
                    let ty = &self.checker.structures[name][&f.name];
                    self.checker.can_initialize(v, ty)
                })
            {
                let mut supplied = 0;
                let mut result = vec![];
                for field in &structure.fields {
                    let ty = &self.checker.structures[name][&field.name];
                    result.push(if eligible(field, public) && supplied < values.len() {
                        let value = self.list(&values[supplied], ty, depth + 1)?;
                        supplied += 1;
                        value
                    } else {
                        self.field_default(field, ty, depth + 1)?
                    });
                }
                return Ok(Initialization {
                    ty: target.clone(),
                    kind: InitializationKind::Aggregate(result),
                });
            }
        }
        if matches!(
            target,
            Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..)
        ) && values.len() == 1
        {
            let actual = self.checker.expression_types.get(&values[0].id);
            if actual.is_some_and(|t| t.convertible_to(target)) {
                return self.value(&values[0], target);
            }
        }
        if let Type::Matrix(scalar, rows, columns) = target {
            let row = Type::Vector(*scalar, *columns);
            if values.len() == usize::from(*rows)
                && values.iter().all(|v| self.checker.can_initialize(v, &row))
            {
                return Ok(Initialization {
                    ty: target.clone(),
                    kind: InitializationKind::Aggregate(
                        values
                            .iter()
                            .map(|v| self.list(v, &row, depth + 1))
                            .collect::<Result<_, _>>()?,
                    ),
                });
            }
        }
        if matches!(target, Type::Vector(..))
            && values.len() > 1
            && values
                .iter()
                .any(|v| matches!(v.kind, ExprKind::InitializerList(_)))
        {
            let types: Vec<_> = values
                .iter()
                .map(|v| {
                    self.checker
                        .expression_types
                        .get(&v.id)
                        .cloned()
                        .unwrap_or(Type::Error)
                })
                .collect();
            if let Some(parameters) = self.checker.numeric_list_types(values, &types, target) {
                return Ok(Initialization {
                    ty: target.clone(),
                    kind: InitializationKind::Aggregate(
                        values
                            .iter()
                            .zip(parameters)
                            .map(|(v, ty)| self.list(v, &ty, depth + 1))
                            .collect::<Result<_, _>>()?,
                    ),
                });
            }
        }
        let mut index = 0;
        let result = self.aggregate(values, &mut index, target, value.span, depth + 1)?;
        if index != values.len() {
            return Err(Diagnostic::error(
                "T021",
                value.span,
                format!("too many initializers for {target}"),
            ));
        }
        Ok(result)
    }

    fn read(
        &mut self,
        values: &[Expr],
        index: &mut usize,
        target: &Type,
        span: Span,
        depth: usize,
    ) -> Result<Initialization, Diagnostic> {
        let Some(value) = values.get(*index) else {
            return self.default(target, span, depth);
        };
        if matches!(value.kind, ExprKind::InitializerList(_)) {
            *index += 1;
            return self.list(value, target, depth);
        }
        let actual = self
            .checker
            .expression_types
            .get(&value.id)
            .unwrap_or(&Type::Error);
        if matches!(target, Type::Scalar(_))
            || !matches!(actual, Type::Scalar(_)) && actual.convertible_to(target)
        {
            *index += 1;
            self.charge(depth, value.span)?;
            self.value(value, target)
        } else {
            self.aggregate(values, index, target, span, depth)
        }
    }

    fn aggregate(
        &mut self,
        values: &[Expr],
        index: &mut usize,
        target: &Type,
        span: Span,
        depth: usize,
    ) -> Result<Initialization, Diagnostic> {
        self.charge(depth, span)?;
        let mut result = vec![];
        match target {
            Type::Scalar(_) => return self.read(values, index, target, span, depth + 1),
            Type::Vector(scalar, width) => {
                // Keep vector operands intact so calls are evaluated once.
                let mut components = 0;
                while components < *width {
                    let actual = values
                        .get(*index)
                        .and_then(|v| self.checker.expression_types.get(&v.id));
                    let count = actual.and_then(Type::width).unwrap_or(1);
                    if components + count > *width {
                        return Err(self.invalid(span, target));
                    }
                    let ty = Type::numeric_shape(*scalar, count);
                    result.push(self.read(values, index, &ty, span, depth + 1)?);
                    components += count;
                }
            }
            Type::Matrix(scalar, rows, columns) => {
                for _ in 0..*rows {
                    result.push(self.read(
                        values,
                        index,
                        &Type::Vector(*scalar, *columns),
                        span,
                        depth + 1,
                    )?);
                }
            }
            Type::Array(element, length) => {
                if *length as usize > self.remaining {
                    return Err(Diagnostic::error(
                        "T020",
                        span,
                        "aggregate initialization size limit exceeded",
                    ));
                }
                for _ in 0..*length {
                    result.push(self.read(values, index, element, span, depth + 1)?);
                }
            }
            Type::Struct(name) => {
                let structure = &self.checker.struct_definitions[name];
                let public = self.checker.public_types.contains(name);
                for field in &structure.fields {
                    let ty = &self.checker.structures[name][&field.name];
                    result.push(if eligible(field, public) && *index < values.len() {
                        self.read(values, index, ty, span, depth + 1)?
                    } else {
                        self.field_default(field, ty, depth + 1)?
                    });
                }
            }
            Type::Error => {
                return Ok(Initialization {
                    ty: Type::Error,
                    kind: InitializationKind::Zero,
                });
            }
            _ => return Err(self.invalid(span, target)),
        }
        Ok(Initialization {
            ty: target.clone(),
            kind: InitializationKind::Aggregate(result),
        })
    }

    fn field_default(
        &mut self,
        field: &Variable,
        target: &Type,
        depth: usize,
    ) -> Result<Initialization, Diagnostic> {
        if let Some(value) = &field.initializer {
            self.charge(depth, field.span)?;
            Ok(Initialization {
                ty: target.clone(),
                kind: InitializationKind::DeclarationDefault(Box::new(value.clone())),
            })
        } else {
            self.default(target, field.span, depth)
        }
    }

    fn default(
        &mut self,
        target: &Type,
        span: Span,
        depth: usize,
    ) -> Result<Initialization, Diagnostic> {
        if matches!(
            target,
            Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..) | Type::Error
        ) {
            self.charge(depth, span)?;
            Ok(Initialization {
                ty: target.clone(),
                kind: InitializationKind::Zero,
            })
        } else {
            self.aggregate(&[], &mut 0, target, span, depth)
        }
    }
}

fn eligible(field: &Variable, public: bool) -> bool {
    !public || field.modifiers.iter().any(|m| m == "public")
}
