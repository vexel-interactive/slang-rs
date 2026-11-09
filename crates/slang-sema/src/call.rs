use crate::{
    Scalar, Type,
    checker::{Checker, Signature},
    intrinsic::intrinsic,
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Expr, ExprKind};

impl Checker {
    pub(crate) fn call(&mut self, callee: &Expr, arguments: &[Expr], span: Span) -> Type {
        let mut types: Vec<Type> = arguments.iter().map(|a| self.expression(a)).collect();
        if let ExprKind::Member { base, name } = &callee.kind {
            if let Some(result) =
                self.member_call(callee, Some(base), name, arguments, &types, span)
            {
                return result;
            }
            return self.resource_call(base, name, arguments, &types, callee.id, span);
        }
        if let ExprKind::Name(name) = &callee.kind
            && self.lookup(name).is_none()
            && let Some(result) = self.member_call(callee, None, name, arguments, &types, span)
        {
            return result;
        }
        if matches!(&callee.kind, ExprKind::Name(name) if matches!(name.as_str(), "vector" | "matrix") && self.lookup(name).is_none() && !(self.functions.contains_key(name) && self.module_symbol_visible(name)))
        {
            self.error(
                "T014",
                callee.span,
                "numeric template constructors require explicit angle brackets or a type alias",
            );
            return Type::Error;
        }
        let constructor = match self.constructor_target(callee) {
            Ok(target) => target,
            Err(error) => {
                self.diagnostics.push(error);
                return Type::Error;
            }
        };
        if let Some(target) = constructor {
            for (argument, ty) in arguments.iter().zip(&mut types) {
                if matches!(ty, Type::ConstantBuffer { .. }) {
                    *ty = self.value_expression(argument);
                }
            }
            if matches!(target, Type::Array(..)) {
                let result = self.array_constructor(&target, arguments, callee.id, span);
                self.expression_types
                    .insert(callee.id, Type::Meta(Box::new(target)));
                return result;
            }
            if let Type::Struct(name) = &target {
                if let Some(function) = self.constructors.get(name).cloned() {
                    let has_copy = self
                        .functions
                        .get(&function)
                        .is_some_and(|s| s.iter().any(|s| s.parameters == [target.clone()]));
                    if types != [target.clone()] || has_copy {
                        let mut selected = callee.clone();
                        selected.kind = ExprKind::Name(function);
                        return self.call(&selected, arguments, span);
                    }
                }
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
                Type::Scalar(_) => {
                    types.len() == 1 && matches!(types[0], Type::Scalar(_) | Type::Vector(_, 1))
                }
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
        if let Some(overloads) = self
            .functions
            .get(name)
            .filter(|_| self.module_symbol_visible(name))
            .cloned()
        {
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
                    .zip(arguments)
                    .map(|((a, b), expr)| {
                        a.literal_conversion_cost(
                            b,
                            slang_syntax::integer_literal(expr).ok().flatten(),
                        )
                    })
                    .sum::<usize>()
            });
            let best = matches.first();
            if let Some(best) = best {
                let score = types
                    .iter()
                    .zip(&best.parameters)
                    .zip(arguments)
                    .map(|((a, b), expr)| {
                        a.literal_conversion_cost(
                            b,
                            slang_syntax::integer_literal(expr).ok().flatten(),
                        )
                    })
                    .sum::<usize>();
                if matches.get(1).is_some_and(|s| {
                    types
                        .iter()
                        .zip(&s.parameters)
                        .zip(arguments)
                        .map(|((a, b), expr)| {
                            a.literal_conversion_cost(
                                b,
                                slang_syntax::integer_literal(expr).ok().flatten(),
                            )
                        })
                        .sum::<usize>()
                        == score
                }) {
                    self.error("T014", span, format!("ambiguous call to '{name}'"));
                }
                if !best.defined {
                    self.error("T014", span, format!("function '{name}' has no definition"));
                }
                return self.finish_user_call(callee, arguments, &types, best, span);
            }
            self.error("T014", span, format!("no matching overload for '{name}'"));
            return Type::Error;
        }
        for (argument, ty) in arguments.iter().zip(&mut types) {
            if matches!(ty, Type::ConstantBuffer { .. }) {
                *ty = self.value_expression(argument);
            }
        }
        if types.contains(&Type::InitializerList)
            && (name != "refract" || types.get(2) != Some(&Type::InitializerList))
            && matches!(
                name.as_str(),
                "dot"
                    | "min"
                    | "max"
                    | "pow"
                    | "clamp"
                    | "lerp"
                    | "cross"
                    | "distance"
                    | "reflect"
                    | "refract"
                    | "faceforward"
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
        if let Some(signature) = crate::intrinsic::bitcast_signature(
            name,
            &types,
            arguments
                .first()
                .and_then(|expr| slang_syntax::integer_literal(expr).ok().flatten()),
        )
        .or_else(|| crate::intrinsic::bitops_signature(name, &types))
        .or_else(|| crate::intrinsic::geometric_signature(name, &types))
        {
            if let Some(start) = signature.outputs_start {
                for argument in &arguments[start..] {
                    self.require_writable(argument);
                }
            }
            self.call_output_parameters.insert(
                crate::span_key(span),
                (
                    name.clone(),
                    (0..arguments.len())
                        .map(|i| signature.outputs_start.is_some_and(|start| i >= start))
                        .collect(),
                ),
            );
            // The signature records explicit input conversions. Output integer
            // signs stay intact: casting an output would destroy its location.
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: signature.parameters,
                    result: Box::new(signature.result.clone()),
                },
            );
            return signature.result;
        }
        if let Some(signature) = crate::intrinsic::atomic_signature(name, &types) {
            self.call_output_parameters.insert(
                crate::span_key(span),
                (
                    name.clone(),
                    (0..arguments.len())
                        .map(|i| i == 0 || signature.output == Some(i))
                        .collect(),
                ),
            );
            for (index, ((argument, actual), target)) in arguments
                .iter()
                .zip(&types)
                .zip(&signature.parameters)
                .enumerate()
            {
                if index == 0 {
                    self.require_atomic_destination(argument);
                } else if signature.output == Some(index) {
                    self.require_writable(argument);
                    if actual != target {
                        self.error(
                            "T014",
                            argument.span,
                            format!("atomic output requires {target}, found {actual}"),
                        );
                    }
                } else {
                    self.coerce_value(argument, target);
                }
            }
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: signature.parameters,
                    result: Box::new(Type::Void),
                },
            );
            return Type::Void;
        }
        if let Some(signature) = crate::intrinsic::output_signature(name, &types) {
            self.call_output_parameters.insert(
                crate::span_key(span),
                (name.clone(), (0..arguments.len()).map(|i| i > 0).collect()),
            );
            for (index, ((argument, actual), target)) in arguments
                .iter()
                .zip(&types)
                .zip(&signature.parameters)
                .enumerate()
            {
                if index == 0 {
                    self.coerce_value(argument, target);
                } else {
                    self.require_writable(argument);
                    if actual != target {
                        self.error(
                            "T014",
                            argument.span,
                            format!("output argument requires {target}, found {actual}"),
                        );
                    }
                }
            }
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: signature.parameters,
                    result: Box::new(signature.result.clone()),
                },
            );
            return signature.result;
        }
        if let Some(result) = crate::intrinsic::derivative_signature(name, &types) {
            self.coerce_value(&arguments[0], &result);
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: vec![result.clone()],
                    result: Box::new(result.clone()),
                },
            );
            return result;
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

    pub(crate) fn constructor_target(&mut self, callee: &Expr) -> Result<Option<Type>, Diagnostic> {
        match &callee.kind {
            ExprKind::TypeApplication(reference) => Ok(Some(self.resolve_type(reference))),
            ExprKind::Name(name) if self.lookup(name).is_none() => Ok(Type::builtin(name)
                .or_else(|| {
                    self.module_symbol_visible(name)
                        .then(|| self.alias_types.get(name).cloned())
                        .flatten()
                })
                .or_else(|| {
                    (self.module_symbol_visible(name) && self.structures.contains_key(name))
                        .then(|| Type::Struct(name.clone()))
                })),
            ExprKind::Index { base, index } => {
                let Some(element) = self.constructor_target(base)? else {
                    return Ok(None);
                };
                let size = self
                    .numeric_shape_value(index)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| {
                        Diagnostic::error(
                            "T002",
                            index.span,
                            "array size must be a nonnegative constant integer",
                        )
                    })?;
                let mut rank = 0;
                let mut ty = &element;
                while let Type::Array(inner, _) = ty {
                    rank += 1;
                    ty = inner;
                }
                if rank >= 32
                    || element == Type::Void
                    || ty.is_resource() && size != 0 && !self.is_empty_type(&element)
                {
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

    pub(crate) fn finish_user_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        types: &[Type],
        best: &Signature,
        span: Span,
    ) -> Type {
        for (argument, target) in arguments.iter().zip(&best.parameters) {
            self.coerce_value(argument, target);
        }
        for (index, (argument, output)) in arguments.iter().zip(&best.output_parameters).enumerate()
        {
            if *output {
                self.require_writable(argument);
                if !types[index].output_convertible_to(&best.parameters[index]) {
                    self.error(
                        "T015",
                        argument.span,
                        format!(
                            "output argument cannot view {} storage as {}",
                            types[index], best.parameters[index]
                        ),
                    );
                }
            }
        }
        self.expression_types.insert(
            callee.id,
            Type::Callable {
                parameters: best.parameters.clone(),
                result: Box::new(best.result.clone()),
            },
        );
        self.call_names.insert(callee.id, best.target_name.clone());
        self.call_output_parameters.insert(
            crate::span_key(span),
            (best.target_name.clone(), best.output_parameters.clone()),
        );
        if arguments.len() < best.parameters.len() {
            let defaults = best.defaults[arguments.len()..]
                .iter()
                .zip(&best.parameters[arguments.len()..])
                .map(|(value, ty)| crate::Initialization {
                    ty: ty.clone(),
                    kind: crate::InitializationKind::DeclarationDefault(Box::new(
                        value.as_ref().unwrap().clone(),
                    )),
                })
                .collect();
            self.default_arguments.insert(callee.id, defaults);
        }
        best.result.clone()
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
