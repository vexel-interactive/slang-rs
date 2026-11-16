use crate::{MethodReceiver, Scalar, TextureDimension, Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Expr, ExprId, ExprKind, TypeArgument};

/// Internal calls retain resource receivers through generic specialization.
pub(crate) fn typed_byte_marker(name: &str) -> bool {
    name.starts_with("@slang.byte.")
}

struct Method {
    parameters: Vec<Type>,
    output_start: usize,
    result: Type,
}

impl Checker {
    pub(crate) fn typed_byte_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        types: &[Type],
        span: Span,
    ) -> Option<Type> {
        // A user generic method with this spelling can make a plain resource
        // call pending too. Resolve that call as a resource before inference.
        if let ExprKind::Member {
            base,
            name,
            static_lookup,
        } = &callee.kind
            && self.pending_generic_calls.contains(&callee.id)
            && let Type::ByteBuffer {
                ..
            } = self.expression(base)
        {
            self.generic_call_names.insert(callee.id, "@slang.byte.legacy".into());
            self.generic_call_types.insert(callee.id, vec![]);
            if *static_lookup {
                self.error("T014", span, "byte-buffer methods require a value receiver");
                return Some(Type::Error);
            }
            return Some(self.resource_call(base, name, arguments, types, callee.id, span));
        }
        let ExprKind::GenericApplication {
            callee: member,
            arguments: supplied,
        } = &callee.kind
        else {
            return None;
        };
        let ExprKind::Member {
            base,
            name,
            static_lookup,
        } = &member.kind
        else {
            return None;
        };
        let receiver = self.expression(base);
        let Type::ByteBuffer {
            writable,
        } = receiver
        else {
            return None;
        };

        if *static_lookup {
            self.error("T014", span, "byte-buffer methods require a value receiver");
            return Some(Type::Error);
        }

        let [TypeArgument::Type(reference)] = supplied.as_slice() else {
            self.error("T014", span, "typed byte-buffer methods require one type argument");
            return Some(Type::Error);
        };
        let ty = self.resolve_type(reference);
        let major = self.reference_matrix_major(reference).unwrap_or(self.default_matrix_major);
        Some(
            self.typed_byte_method(
                base, name, arguments, types, callee.id, span, writable, ty, major,
            ),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn typed_byte_method(
        &mut self,
        base: &Expr,
        name: &str,
        arguments: &[Expr],
        types: &[Type],
        callee: ExprId,
        span: Span,
        writable: bool,
        ty: Type,
        major: slang_layout::MatrixMajor,
    ) -> Type {
        let load = matches!(name, "Load" | "LoadAligned");
        let store = matches!(name, "Store" | "StoreAligned");
        let marker = format!(
            "@slang.byte.{}.{}",
            if load {
                "load"
            } else {
                "store"
            },
            if major == slang_layout::MatrixMajor::Row {
                "row"
            } else {
                "column"
            }
        );

        if self.pending_generic_calls.contains(&callee) {
            self.generic_call_names.insert(callee, marker.clone());
            self.generic_call_types.insert(callee, vec![]);
        }

        let valid_arity = match name {
            "Load" => arguments.len() == 1,
            "LoadAligned" => matches!(arguments.len(), 1 | 2),
            "Store" => matches!(arguments.len(), 2 | 3),
            "StoreAligned" => arguments.len() == 2,
            _ => false,
        };
        if !valid_arity || (!load && !store) || (store && !writable) {
            self.error(
                "T014",
                span,
                format!("unsupported method or arguments for byte buffer.{name}"),
            );
            return Type::Error;
        }

        if ty == Type::Error || types.contains(&Type::Error) {
            return Type::Error;
        }
        if !self.byte_data_type(&ty, &mut Default::default(), &mut Default::default()) {
            self.error(
                "T014",
                span,
                format!("typed byte-buffer access requires a data type, found {ty}"),
            );
            return Type::Error;
        }

        let uint = Type::Scalar(Scalar::Uint);
        let mut parameters = vec![uint.clone()];
        if store {
            parameters.push(ty.clone());
        }
        let alignment_index = if load {
            1
        } else {
            2
        };
        let natural = self.byte_alignment(&ty, &mut Default::default(), &mut Default::default());
        if name.ends_with("Aligned")
            && arguments.len() == alignment_index
            && let Some(alignment) = natural
            && self
                .numeric_shape_value(&arguments[0])
                .is_some_and(|address| address.rem_euclid(i64::from(alignment)) != 0)
        {
            self.error(
                "T014",
                arguments[0].span,
                "byte-buffer address does not satisfy the type's natural alignment",
            );
        }
        if let Some(alignment) = arguments.get(alignment_index) {
            parameters.push(uint);
            let value = self.numeric_shape_value(alignment);
            match value {
                Some(0) => {}
                Some(value)
                    if value > 0
                        && value <= u32::MAX as i64
                        && (value as u32).is_power_of_two() =>
                {
                    if natural.is_some_and(|natural| value < natural as i64) {
                        self.error(
                            "T014",
                            alignment.span,
                            "byte-buffer alignment is smaller than the type's natural alignment",
                        );
                    }
                    if self
                        .numeric_shape_value(&arguments[0])
                        .is_some_and(|address| address.rem_euclid(value) != 0)
                    {
                        self.error(
                            "T014",
                            arguments[0].span,
                            "byte-buffer address does not satisfy the promised alignment",
                        );
                    }
                }
                Some(_) => self.error(
                    "T014",
                    alignment.span,
                    "byte-buffer alignment must be zero or a power of two",
                ),
                None => self.error(
                    "T014",
                    alignment.span,
                    "byte-buffer alignment must be a compile-time integer",
                ),
            }
        }

        for (argument, parameter) in arguments.iter().zip(&parameters) {
            self.coerce_value(argument, parameter);
        }
        let result = if load {
            ty
        } else {
            Type::Void
        };
        self.call_names.insert(callee, marker.clone());
        self.method_receivers.insert(callee, MethodReceiver::Explicit(Box::new(base.clone())));
        self.call_output_parameters
            .insert(crate::span_key(span), (marker, vec![false; arguments.len()]));
        self.expression_types.insert(
            callee,
            Type::Callable {
                parameters,
                result: Box::new(result.clone()),
            },
        );
        result
    }

    fn byte_data_type(
        &self,
        ty: &Type,
        path: &mut std::collections::BTreeSet<String>,
        cache: &mut std::collections::BTreeMap<String, bool>,
    ) -> bool {
        match ty {
            Type::Struct(name) if self.symbolic_types.contains(name) => true,
            Type::Struct(name) if cache.contains_key(name) => cache[name],
            Type::Struct(name) if path.insert(name.clone()) => {
                let valid = self.structures.get(name).is_some_and(|fields| {
                    fields.values().all(|field| self.byte_data_type(field, path, cache))
                });
                path.remove(name);
                cache.insert(name.clone(), valid);
                valid
            }
            Type::Array(element, _) => self.byte_data_type(element, path, cache),
            Type::Void | Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..) => true,
            _ => false,
        }
    }

    fn byte_alignment(
        &self,
        ty: &Type,
        path: &mut std::collections::BTreeSet<String>,
        cache: &mut std::collections::BTreeMap<String, Option<u32>>,
    ) -> Option<u32> {
        match ty {
            Type::Void => Some(1),
            Type::Scalar(scalar) | Type::Vector(scalar, _) | Type::Matrix(scalar, _, _) => {
                Some(match scalar {
                    Scalar::Bool => 4,
                    _ => u32::from(scalar.bit_width() / 8),
                })
            }
            Type::Array(element, _) => self.byte_alignment(element, path, cache),
            Type::Struct(name) if self.symbolic_types.contains(name) => None,
            Type::Struct(name) if cache.contains_key(name) => cache[name],
            Type::Struct(name) if path.insert(name.clone()) => {
                let result = self.structures.get(name).and_then(|fields| {
                    fields
                        .values()
                        .map(|field| self.byte_alignment(field, path, cache))
                        .try_fold(1, |a, b| b.map(|b| a.max(b)))
                });
                path.remove(name);
                cache.insert(name.clone(), result);
                result
            }
            _ => None,
        }
    }

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

        if let Type::ByteBuffer {
            writable,
        } = receiver
            && matches!(name, "Store" | "StoreAligned")
            && let Some(ty) = types.get(1)
            && !(name == "Store"
                && arguments.len() == 2
                && *ty.value_type() == Type::Scalar(Scalar::Uint))
        {
            let major =
                self.expression_matrix_major(&arguments[1]).unwrap_or(self.default_matrix_major);
            return self.typed_byte_method(
                base,
                name,
                arguments,
                types,
                callee,
                span,
                writable,
                ty.value_type().clone(),
                major,
            );
        }

        let types: Vec<_> = types.iter().map(|ty| ty.value_type().clone()).collect();
        let Some(method) = signature(&receiver, name, &types) else {
            self.error(
                "T014",
                span,
                format!("unsupported method or arguments for {receiver}.{name}"),
            );
            return Type::Error;
        };

        self.call_output_parameters.insert(
            crate::span_key(span),
            (name.to_owned(), (0..arguments.len()).map(|i| i >= method.output_start).collect()),
        );

        for (index, ((actual, expected), argument)) in
            types.iter().zip(&method.parameters).zip(arguments).enumerate()
        {
            if index >= method.output_start {
                if matches!(
                    self.expression_types.get(&argument.id),
                    Some(Type::ConstantBuffer { .. })
                ) {
                    self.error(
                        "T015",
                        argument.span,
                        "constant-buffer contents are not writable output locations",
                    );
                }

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
        Type::ByteBuffer {
            writable,
        } => byte_buffer(name, types, *writable),
        Type::Buffer {
            element,
            ..
        } => match name {
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
            let sampler = Type::Sampler {
                comparison: false,
            };
            let comparison = Type::Sampler {
                comparison: true,
            };
            let floating = matches!(element.scalar(), Some(Scalar::Half | Scalar::Float));

            match name {
                "Load" if *dimension != TextureDimension::Cube => input(
                    vec![Type::numeric_shape(Scalar::Int, width + u8::from(!writable))],
                    *element.clone(),
                ),
                "Sample" if !writable && floating => {
                    input(vec![sampler, coordinate], *element.clone())
                }
                "SampleLevel" | "SampleBias" if !writable && floating => {
                    input(vec![sampler, coordinate, Type::Scalar(Scalar::Float)], *element.clone())
                }
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
