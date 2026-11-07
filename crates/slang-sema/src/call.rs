use crate::{Scalar, Type, checker::Checker, intrinsic::intrinsic};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Expr, ExprKind};

impl Checker {
    pub(crate) fn call(&mut self, callee: &Expr, arguments: &[Expr], span: Span) -> Type {
        let mut types: Vec<Type> = arguments.iter().map(|a| self.expression(a)).collect();
        if let ExprKind::Member { base, name } = &callee.kind {
            return self.resource_call(base, name, arguments, &types, callee.id, span);
        }
        let constructor = match self.constructor_target(callee) {
            Ok(target) => target,
            Err(error) => {
                self.diagnostics.push(error);
                return Type::Error;
            }
        };
        if let Some(target) = constructor {
            if matches!(target, Type::Array(..)) {
                let result = self.array_constructor(&target, arguments, callee.id, span);
                self.expression_types
                    .insert(callee.id, Type::Meta(Box::new(target)));
                return result;
            }
            if let Type::Struct(name) = &target {
                let result = self.struct_constructor(name, arguments, &types, callee.id, span);
                self.expression_types
                    .insert(callee.id, Type::Meta(Box::new(target)));
                return result;
            }
            if types.contains(&Type::InitializerList) {
                if arguments.len() == 1 && matches!(target, Type::Vector(..) | Type::Matrix(..)) {
                    let scalar = Type::Scalar(target.scalar().unwrap());
                    self.coerce_value(&arguments[0], &scalar);
                    types[0] = scalar;
                } else {
                    let Some(selected) = self.numeric_list_types(arguments, &types, &target) else {
                        self.error(
                            "T014",
                            span,
                            "invalid or ambiguous initializer-list constructor arguments",
                        );
                        return Type::Error;
                    };
                    for (argument, target) in arguments.iter().zip(&selected) {
                        if matches!(argument.kind, ExprKind::InitializerList(_)) {
                            self.coerce_value(argument, target);
                        }
                    }
                    types = selected;
                }
            }
            let valid = match &target {
                Type::Scalar(_) => types.len() == 1 && matches!(types[0], Type::Scalar(_)),
                Type::Vector(scalar, width) => {
                    let components: Option<u16> =
                        types.iter().map(|t| t.width().map(u16::from)).sum();
                    let scalar_types_valid = types.iter().all(|t| {
                        t.scalar()
                            .is_some_and(|s| (s == Scalar::Bool) == (*scalar == Scalar::Bool))
                    });
                    scalar_types_valid
                        && (components == Some(u16::from(*width))
                            || types.len() == 1 && types[0].width() == Some(1))
                }
                Type::Matrix(scalar, rows, columns) => {
                    let components: Option<u16> = types.iter().map(Type::components).sum();
                    let scalar_types_valid = types.iter().all(|t| {
                        t.scalar()
                            .is_some_and(|s| (s == Scalar::Bool) == (*scalar == Scalar::Bool))
                    });
                    scalar_types_valid
                        && (types.len() == 1
                            && (matches!(types[0], Type::Scalar(_))
                                || matches!(types[0], Type::Matrix(_, r, c) if r == *rows && c == *columns))
                            || types.iter().all(|t| t.width().is_some())
                                && components == Some(u16::from(*rows) * u16::from(*columns)))
                }
                _ => false,
            };
            if !valid {
                self.error(
                    "T014",
                    span,
                    format!("invalid constructor arguments for {target}"),
                );
            }
            self.expression_types
                .insert(callee.id, Type::Meta(Box::new(target.clone())));
            if matches!(
                target,
                Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..)
            ) {
                self.call_names.insert(callee.id, target.to_string());
            }
            return target;
        }
        let ExprKind::Name(name) = &callee.kind else {
            self.error("T001", span, "only named function calls are implemented");
            return Type::Error;
        };
        if self.lookup(name).is_some() {
            self.error("T014", span, format!("value '{name}' is not callable"));
            return Type::Error;
        }
        if let Some(overloads) = self.functions.get(name).cloned() {
            let owner = self
                .symbols
                .get(name)
                .map_or(slang_module::ModuleId(0), |o| o.module);
            let mut matches: Vec<_> = overloads
                .into_iter()
                .filter(|s| {
                    (s.public || owner == self.current_module)
                        && types.len() <= s.parameters.len()
                        && s.defaults[types.len()..].iter().all(Option::is_some)
                        && types
                            .iter()
                            .zip(&s.parameters)
                            .all(|(a, b)| *a == Type::InitializerList || a.convertible_to(b))
                })
                .collect();
            matches.sort_by_key(|s| {
                types
                    .iter()
                    .zip(&s.parameters)
                    .filter(|(a, b)| **a != Type::InitializerList && a != b)
                    .count()
            });
            let best = matches.first();
            if let Some(best) = best {
                let score = types
                    .iter()
                    .zip(&best.parameters)
                    .filter(|(a, b)| **a != Type::InitializerList && a != b)
                    .count();
                if matches.get(1).is_some_and(|s| {
                    types
                        .iter()
                        .zip(&s.parameters)
                        .filter(|(a, b)| **a != Type::InitializerList && a != b)
                        .count()
                        == score
                }) {
                    self.error("T014", span, format!("ambiguous call to '{name}'"));
                }
                if !best.defined {
                    self.error("T014", span, format!("function '{name}' has no definition"));
                }
                for (argument, output) in arguments.iter().zip(&best.output_parameters) {
                    if *output {
                        self.require_writable(argument);
                    }
                }
                for (argument, target) in arguments.iter().zip(&best.parameters) {
                    self.coerce_value(argument, target);
                }
                self.expression_types.insert(
                    callee.id,
                    Type::Callable {
                        parameters: best.parameters.clone(),
                        result: Box::new(best.result.clone()),
                    },
                );
                self.call_names.insert(callee.id, best.target_name.clone());
                if arguments.len() < best.parameters.len() {
                    let defaults = best.defaults[arguments.len()..]
                        .iter()
                        .zip(&best.parameters[arguments.len()..])
                        .map(|(value, ty)| crate::Initialization {
                            ty: ty.clone(),
                            kind: crate::InitializationKind::Value(Box::new(
                                value.as_ref().unwrap().clone(),
                            )),
                        })
                        .collect();
                    self.default_arguments.insert(callee.id, defaults);
                }
                return best.result.clone();
            }
            self.error("T014", span, format!("no matching overload for '{name}'"));
            return Type::Error;
        }
        if types.contains(&Type::InitializerList)
            && matches!(
                name.as_str(),
                "dot" | "min" | "max" | "pow" | "clamp" | "lerp"
            )
            && let Some(context) = types
                .iter()
                .filter(|t| **t != Type::InitializerList)
                .cloned()
                .reduce(|a, b| Type::common(&a, &b).unwrap_or(Type::Error))
            && context != Type::Error
        {
            for (argument, ty) in arguments.iter().zip(&mut types) {
                if *ty == Type::InitializerList {
                    self.coerce_value(argument, &context);
                    *ty = context.clone();
                }
            }
        }
        if let Some(result) = intrinsic(name, &types) {
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: types,
                    result: Box::new(result.clone()),
                },
            );
            return result;
        }
        self.error(
            "T014",
            span,
            format!("unknown function or unsupported intrinsic '{name}'"),
        );
        Type::Error
    }

    fn constructor_target(&self, callee: &Expr) -> Result<Option<Type>, Diagnostic> {
        match &callee.kind {
            ExprKind::Name(name) if self.lookup(name).is_none() => Ok(Type::builtin(name)
                .or_else(|| self.alias_types.get(name).cloned())
                .or_else(|| {
                    self.structures
                        .contains_key(name)
                        .then(|| Type::Struct(name.clone()))
                })),
            ExprKind::Index { base, index } => {
                let Some(element) = self.constructor_target(base)? else {
                    return Ok(None);
                };
                let ExprKind::Integer(text) = &index.kind else {
                    return Err(Diagnostic::error(
                        "T002",
                        index.span,
                        "array constructor size must be a positive decimal literal",
                    ));
                };
                let size = text
                    .trim_end_matches(['u', 'U'])
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        Diagnostic::error("T002", index.span, "invalid array constructor size")
                    })?;
                let mut rank = 0;
                let mut ty = &element;
                while let Type::Array(inner, _) = ty {
                    rank += 1;
                    ty = inner;
                }
                if rank >= 32 || element == Type::Void || ty.is_resource() {
                    return Err(Diagnostic::error(
                        "T002",
                        callee.span,
                        "invalid or excessively nested array constructor type",
                    ));
                }
                Ok(Some(Type::Array(Box::new(element), size)))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn numeric_list_types(
        &self,
        arguments: &[Expr],
        types: &[Type],
        target: &Type,
    ) -> Option<Vec<Type>> {
        if !matches!(target, Type::Vector(..) | Type::Matrix(..)) {
            return None;
        }
        let components = target.components()?;
        if arguments.len() > usize::from(components) {
            return None;
        }
        let scalar = target.scalar()?;
        let choices: Vec<Vec<Type>> = arguments
            .iter()
            .zip(types)
            .map(|(argument, ty)| {
                if *ty == Type::InitializerList {
                    (1..=4)
                        .map(|width| Type::numeric_shape(scalar, width))
                        .filter(|ty| self.can_initialize(argument, ty))
                        .collect()
                } else {
                    vec![ty.clone()]
                }
            })
            .collect();
        let mut candidates = vec![];
        constructor_partitions(&choices, components, &mut vec![], &mut candidates);
        (candidates.len() == 1).then(|| candidates.remove(0))
    }
}

fn constructor_partitions(
    choices: &[Vec<Type>],
    remaining: u16,
    selected: &mut Vec<Type>,
    candidates: &mut Vec<Vec<Type>>,
) {
    if candidates.len() >= 2 {
        return;
    }
    if choices.is_empty() {
        if remaining == 0 {
            candidates.push(selected.clone());
        }
        return;
    }
    for ty in &choices[0] {
        if let Some(width) = ty.width().map(u16::from)
            && width <= remaining
        {
            selected.push(ty.clone());
            constructor_partitions(&choices[1..], remaining - width, selected, candidates);
            selected.pop();
        }
    }
}
