use crate::{Scalar, Type, checker::Checker, intrinsic::intrinsic};
use slang_source::Span;
use slang_syntax::{Expr, ExprKind};

impl Checker {
    pub(crate) fn call(&mut self, callee: &Expr, arguments: &[Expr], span: Span) -> Type {
        let types: Vec<Type> = arguments.iter().map(|a| self.expression(a)).collect();
        let ExprKind::Name(name) = &callee.kind else {
            self.error("T001", span, "only named function calls are implemented");
            return Type::Error;
        };
        if self.lookup(name).is_some() {
            self.error("T014", span, format!("value '{name}' is not callable"));
            return Type::Error;
        }
        let constructor = Type::builtin(name).or_else(|| {
            self.structures
                .contains_key(name)
                .then(|| Type::Struct(name.clone()))
        });
        if let Some(target) = constructor {
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
                Type::Struct(structure) => {
                    // Struct constructor field order is not implemented; reject explicitly.
                    self.error(
                        "T001",
                        span,
                        format!("constructor for struct '{structure}' is not implemented"),
                    );
                    false
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
            return target;
        }
        if let Some(overloads) = self.functions.get(name).cloned() {
            let mut matches: Vec<_> = overloads
                .into_iter()
                .filter(|s| {
                    s.parameters.len() == types.len()
                        && types
                            .iter()
                            .zip(&s.parameters)
                            .all(|(a, b)| a.convertible_to(b))
                })
                .collect();
            matches.sort_by_key(|s| {
                types
                    .iter()
                    .zip(&s.parameters)
                    .filter(|(a, b)| a != b)
                    .count()
            });
            let best = matches.first();
            if let Some(best) = best {
                let score = types
                    .iter()
                    .zip(&best.parameters)
                    .filter(|(a, b)| a != b)
                    .count();
                if matches.get(1).is_some_and(|s| {
                    types
                        .iter()
                        .zip(&s.parameters)
                        .filter(|(a, b)| a != b)
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
                self.expression_types.insert(
                    callee.id,
                    Type::Callable {
                        parameters: best.parameters.clone(),
                        result: Box::new(best.result.clone()),
                    },
                );
                return best.result.clone();
            }
            self.error("T014", span, format!("no matching overload for '{name}'"));
            return Type::Error;
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
}
