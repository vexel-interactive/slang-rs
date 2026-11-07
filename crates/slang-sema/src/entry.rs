use crate::{
    Scalar, ShaderStage, Type,
    checker::{Checker, EntryPoint},
};
use slang_syntax::{Expr, ExprKind, Function, Module};

impl Checker {
    pub(crate) fn entry_point(
        &mut self,
        function: &Function,
        module: &Module,
    ) -> Option<EntryPoint> {
        let mut stage = None;
        let mut dimensions = None;
        for attribute in &function.attributes {
            match attribute.name.as_str() {
                "shader" => {
                    if stage.is_some() {
                        self.error("T006", attribute.span, "duplicate shader attribute");
                    }
                    if let [
                        Expr {
                            kind: ExprKind::String(value),
                            ..
                        },
                    ] = attribute.arguments.as_slice()
                    {
                        stage = match value.as_str() {
                            "\"compute\"" => Some(ShaderStage::Compute),
                            "\"vertex\"" => Some(ShaderStage::Vertex),
                            "\"fragment\"" | "\"pixel\"" => Some(ShaderStage::Fragment),
                            _ => {
                                self.error("T006", attribute.span, "unsupported shader stage");
                                None
                            }
                        };
                    } else {
                        self.error("T006", attribute.span, "shader requires a stage string");
                    }
                }
                "numthreads" => {
                    if dimensions.is_some() {
                        self.error("T006", attribute.span, "duplicate numthreads attribute");
                    }
                    let values: Option<Vec<u32>> = attribute
                        .arguments
                        .iter()
                        .map(|expr| {
                            let ty = self.expression(expr);
                            let value = match ty {
                                Type::Scalar(Scalar::Int | Scalar::Uint | Scalar::Bool) => {
                                    self.integer_constant(expr)
                                }
                                Type::Scalar(Scalar::Half | Scalar::Float | Scalar::Double) => {
                                    self.constant_conversion(expr)
                                }
                                _ => None,
                            }?;
                            u32::try_from(value).ok().filter(|value| *value > 0)
                        })
                        .collect();
                    if let Some(values) = values.filter(|v| v.len() == 3) {
                        let dims = [values[0], values[1], values[2]];
                        if dims.contains(&0)
                            || dims[0] > 1024
                            || dims[1] > 1024
                            || dims[2] > 64
                            || dims.iter().map(|&v| u64::from(v)).product::<u64>() > 1024
                        {
                            self.error("T006", attribute.span, "invalid compute thread-group size");
                        }
                        dimensions = Some(dims);
                    } else {
                        self.error(
                            "T006",
                            attribute.span,
                            "numthreads requires three positive scalar compile-time constants",
                        );
                    }
                }
                _ => self.error(
                    "T001",
                    attribute.span,
                    format!("attribute '{}' is not implemented", attribute.name),
                ),
            }
        }
        if stage.is_none() && dimensions.is_none() {
            return None;
        }
        if self
            .functions
            .get(&function.name)
            .is_some_and(|overloads| overloads.len() > 1)
        {
            self.error(
                "T006",
                function.span,
                "overloaded shader entry points are not implemented",
            );
        }
        if function.body.is_none() {
            self.error("T006", function.span, "shader entry point must have a body");
        }
        let stage = stage.unwrap_or(ShaderStage::Compute);
        if stage != ShaderStage::Compute {
            if dimensions.is_some() {
                self.error(
                    "T006",
                    function.span,
                    "numthreads is only valid for compute shaders",
                );
            }
            self.graphics_interface(function, module, stage);
            return Some(EntryPoint {
                name: function.name.clone(),
                stage,
                thread_group_size: None,
            });
        }
        if self.resolve_type(&function.return_type) != Type::Void {
            self.error(
                "T006",
                function.span,
                "compute entry point must return void",
            );
        }
        if function.semantic.is_some() {
            self.error(
                "T006",
                function.span,
                "compute entry point cannot have a return semantic",
            );
        }
        for parameter in &function.parameters {
            let expected = match parameter.semantic.as_deref() {
                Some("SV_DispatchThreadID" | "SV_GroupID" | "SV_GroupThreadID") => {
                    Type::Vector(Scalar::Uint, 3)
                }
                Some("SV_GroupIndex") => Type::Scalar(Scalar::Uint),
                _ => {
                    self.error(
                        "T006",
                        parameter.span,
                        "unsupported or missing compute parameter semantic",
                    );
                    Type::Error
                }
            };
            if self.resolve_type(&parameter.ty) != expected && expected != Type::Error {
                self.error(
                    "T006",
                    parameter.span,
                    format!("compute semantic requires {expected}"),
                );
            }
            if parameter
                .modifiers
                .iter()
                .any(|m| m == "out" || m == "inout")
            {
                self.error(
                    "T006",
                    parameter.span,
                    "compute system values must be input parameters",
                );
            }
        }
        let dimensions = dimensions.unwrap_or_else(|| {
            self.error(
                "T006",
                function.span,
                "compute entry point requires numthreads",
            );
            [1, 1, 1]
        });
        Some(EntryPoint {
            name: function.name.clone(),
            stage,
            thread_group_size: Some(dimensions),
        })
    }
}
