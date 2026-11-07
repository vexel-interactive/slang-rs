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
                self.require_conversion(actual, expected, argument.span);
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
                "SampleLevel" if !writable && floating => input(
                    vec![sampler, coordinate, Type::Scalar(Scalar::Float)],
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
