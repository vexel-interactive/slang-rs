use crate::{Scalar, TextureDimension, Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Expr, ExprId};

struct Method {
    parameters: Vec<Type>,
    output_start: usize,
    result: Type,
}

impl Checker {
    pub(crate) fn resource_call(
        &mut self,
        base: &Expr,
        name: &str,
        arguments: &[Expr],
        types: &[Type],
        callee: ExprId,
        span: Span,
    ) -> Type {
        let receiver = self.expression(base);
        if receiver == Type::Error || types.contains(&Type::Error) {
            return Type::Error;
        }
        let Some(method) = signature(&receiver, name, types) else {
            self.error(
                "T014",
                span,
                format!("unsupported method or arguments for {receiver}.{name}"),
            );
            return Type::Error;
        };
        for (index, ((actual, expected), argument)) in types
            .iter()
            .zip(&method.parameters)
            .zip(arguments)
            .enumerate()
        {
            if index >= method.output_start {
                self.require_writable(argument);
                if actual != expected {
                    self.error(
                        "T014",
                        argument.span,
                        format!("output argument requires {expected}, found {actual}"),
                    );
                }
            } else {
                self.coerce_value(argument, expected);
            }
        }
        self.expression_types.insert(
            callee,
            Type::Callable {
                parameters: method.parameters,
                result: Box::new(method.result.clone()),
            },
        );
        method.result
    }
}

fn signature(receiver: &Type, name: &str, types: &[Type]) -> Option<Method> {
    let input = |parameters: Vec<Type>, result: Type| {
        (parameters.len() == types.len()).then_some(Method {
            output_start: parameters.len(),
            parameters,
            result,
        })
    };
    match receiver {
        Type::ByteBuffer { writable } => byte_buffer(name, types, *writable),
        Type::Buffer { element, .. } => match name {
            "Load" => input(vec![Type::Scalar(Scalar::Int)], *element.clone()),
            "GetDimensions" if types.len() == 2 && types[0] == Type::Scalar(Scalar::Uint) => {
                dimensions(types, 0, 2)
            }
            _ => None,
        },
        Type::Texture {
            element,
            dimension,
            writable,
        } => {
            let width = dimension.coordinates();
            let coordinate = Type::numeric_shape(Scalar::Float, width);
            let sampler = Type::Sampler { comparison: false };
            let comparison = Type::Sampler { comparison: true };
            let floating = matches!(element.scalar(), Some(Scalar::Half | Scalar::Float));
            match name {
                "Load" if *dimension != TextureDimension::Cube => input(
                    vec![Type::numeric_shape(
                        Scalar::Int,
                        width + u8::from(!writable),
                    )],
                    *element.clone(),
                ),
                "Sample" if !writable && floating => {
                    input(vec![sampler, coordinate], *element.clone())
                }
                "SampleLevel" | "SampleBias" if !writable && floating => input(
                    vec![sampler, coordinate, Type::Scalar(Scalar::Float)],
                    *element.clone(),
                ),
                "SampleGrad" if !writable && floating => input(
                    vec![sampler, coordinate.clone(), coordinate.clone(), coordinate],
                    *element.clone(),
                ),
                "SampleCmp" | "SampleCmpLevelZero"
                    if !writable && floating && *dimension != TextureDimension::D3 =>
                {
                    input(
                        vec![comparison, coordinate, Type::Scalar(Scalar::Float)],
                        Type::Scalar(Scalar::Float),
                    )
                }
                "GetDimensions" if types.len() == dimension.dimensions() => {
                    dimensions(types, 0, dimension.dimensions())
                }
                "GetDimensions" if !writable && types.len() == dimension.dimensions() + 2 => {
                    dimensions(types, 1, dimension.dimensions() + 1)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn byte_buffer(name: &str, types: &[Type], writable: bool) -> Option<Method> {
    let uint = Type::Scalar(Scalar::Uint);
    let width = |suffix: &str| match suffix {
        "" => Some(1),
        "2" => Some(2),
        "3" => Some(3),
        "4" => Some(4),
        _ => None,
    };
    if let Some(suffix) = name.strip_prefix("Load") {
        let components = width(suffix)?;
        return (types.len() == 1).then_some(Method {
            parameters: vec![uint],
            output_start: 1,
            result: Type::numeric_shape(Scalar::Uint, components),
        });
    }
    if let Some(suffix) = name.strip_prefix("Store") {
        let components = width(suffix)?;
        return (writable && types.len() == 2).then_some(Method {
            parameters: vec![uint, Type::numeric_shape(Scalar::Uint, components)],
            output_start: 2,
            result: Type::Void,
        });
    }
    if name == "GetDimensions" && types == [uint.clone()] {
        return dimensions(types, 0, 1);
    }
    if writable {
        let inputs = match name {
            "InterlockedAdd"
            | "InterlockedAnd"
            | "InterlockedOr"
            | "InterlockedXor"
            | "InterlockedExchange" => 2,
            "InterlockedCompareExchange" => 3,
            _ => return None,
        };
        if types.len() == inputs + 1 {
            return Some(Method {
                parameters: vec![uint; inputs + 1],
                output_start: inputs,
                result: Type::Void,
            });
        }
    }
    None
}

fn dimensions(types: &[Type], input_count: usize, outputs: usize) -> Option<Method> {
    let scalar = match types.get(input_count)? {
        Type::Scalar(Scalar::Uint) => Scalar::Uint,
        Type::Scalar(Scalar::Float) => Scalar::Float,
        _ => return None,
    };
    let mut parameters = vec![Type::Scalar(Scalar::Uint); input_count];
    parameters.extend(vec![Type::Scalar(scalar); outputs]);
    Some(Method {
        parameters,
        output_start: input_count,
        result: Type::Void,
    })
}
