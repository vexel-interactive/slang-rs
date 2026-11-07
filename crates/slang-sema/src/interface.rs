use crate::{Scalar, ShaderStage, Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Declaration, Function, Module};
use std::collections::BTreeSet;

struct Interface<'a> {
    module: &'a Module,
    stage: ShaderStage,
    output: bool,
    semantics: BTreeSet<String>,
}

impl Checker {
    pub(crate) fn check_interpolation(&mut self, modifiers: &[String], span: Span) {
        if modifiers.iter().filter(|m| *m == "nointerpolation").count() > 1 {
            self.error("T001", span, "duplicate nointerpolation modifier");
        }
    }

    pub(crate) fn graphics_interface(
        &mut self,
        function: &Function,
        module: &Module,
        stage: ShaderStage,
    ) {
        let mut inputs = Interface {
            module,
            stage,
            output: false,
            semantics: BTreeSet::new(),
        };
        let mut outputs = Interface {
            module,
            stage,
            output: true,
            semantics: BTreeSet::new(),
        };
        for parameter in &function.parameters {
            let ty = self.resolve_type(&parameter.ty);
            let input = !parameter.modifiers.iter().any(|m| m == "out");
            let output = parameter
                .modifiers
                .iter()
                .any(|m| m == "out" || m == "inout");
            let flat = parameter.modifiers.iter().any(|m| m == "nointerpolation");
            if input {
                self.interface_value(
                    &ty,
                    parameter.semantic.as_deref(),
                    parameter.span,
                    &mut inputs,
                    0,
                    flat,
                );
            }
            if output {
                self.interface_value(
                    &ty,
                    parameter.semantic.as_deref(),
                    parameter.span,
                    &mut outputs,
                    0,
                    flat,
                );
            }
        }
        let result = self.resolve_type(&function.return_type);
        if result != Type::Void {
            self.interface_value(
                &result,
                function.semantic.as_deref(),
                function.span,
                &mut outputs,
                0,
                false,
            );
        } else if function.semantic.is_some() {
            self.error(
                "T006",
                function.span,
                "a void return cannot have a semantic",
            );
        }
        if stage == ShaderStage::Vertex && !outputs.semantics.contains("SV_POSITION0") {
            self.error("T006", function.span, "vertex output requires SV_Position");
        }
    }

    fn interface_value(
        &mut self,
        ty: &Type,
        semantic: Option<&str>,
        span: Span,
        interface: &mut Interface<'_>,
        depth: usize,
        flat: bool,
    ) {
        if depth >= 128 {
            self.error("T006", span, "shader interface nesting exceeds 128 levels");
            return;
        }
        if let Type::Struct(name) = ty {
            if semantic.is_some() {
                self.error(
                    "T001",
                    span,
                    "semantics on whole structs are not implemented; use field semantics",
                );
                return;
            }
            if let Some(structure) = interface.module.declarations.iter().find_map(|d| match d {
                Declaration::Struct(s) if &s.name == name => Some(s),
                _ => None,
            }) {
                for field in &structure.fields {
                    let ty = self.resolve_type(&field.ty);
                    self.interface_value(
                        &ty,
                        field.semantic.as_deref(),
                        field.span,
                        interface,
                        depth + 1,
                        flat || field.modifiers.iter().any(|m| m == "nointerpolation"),
                    );
                }
            }
            return;
        }
        if *ty == Type::Error {
            return;
        }
        if !matches!(ty, Type::Scalar(_) | Type::Vector(..)) {
            self.error(
                "T001",
                span,
                "shader interface leaves currently require scalar or vector types",
            );
            return;
        }
        let Some(semantic) = semantic else {
            self.error("T006", span, "shader interface leaf requires a semantic");
            return;
        };
        let (name, index) = semantic_parts(semantic);
        let key = format!("{name}{index}");
        if !interface.semantics.insert(key) {
            self.error(
                "T006",
                span,
                format!("duplicate shader interface semantic '{semantic}'"),
            );
        }
        let valid = match (interface.stage, interface.output, name.as_str(), index) {
            (ShaderStage::Vertex, true, "SV_POSITION", 0)
            | (ShaderStage::Fragment, false, "SV_POSITION", 0) => {
                *ty == Type::Vector(Scalar::Float, 4)
            }
            (ShaderStage::Vertex, false, "SV_VERTEXID" | "SV_INSTANCEID", 0)
            | (ShaderStage::Fragment, false, "SV_PRIMITIVEID" | "SV_SAMPLEINDEX", 0)
            | (ShaderStage::Fragment, _, "SV_COVERAGE", 0) => *ty == Type::Scalar(Scalar::Uint),
            (ShaderStage::Fragment, false, "SV_ISFRONTFACE", 0) => {
                *ty == Type::Scalar(Scalar::Bool)
            }
            (ShaderStage::Fragment, true, "SV_TARGET", 0..=7) => ty.numeric(),
            (
                ShaderStage::Fragment,
                true,
                "SV_DEPTH" | "SV_DEPTHGREATEREQUAL" | "SV_DEPTHLESSEQUAL",
                0,
            ) => *ty == Type::Scalar(Scalar::Float),
            _ if !name.starts_with("SV_") => {
                ty.numeric()
                    && !(interface.stage == ShaderStage::Fragment && interface.output)
                    && !(interface.stage == ShaderStage::Fragment && ty.integral() && !flat)
            }
            _ => false,
        };
        if !valid {
            self.error("T006", span, format!("unsupported semantic '{semantic}' or incompatible type {ty} for this shader interface"));
        }
    }
}

// HLSL semantics are case-insensitive and an absent numeric suffix means zero.
fn semantic_parts(semantic: &str) -> (String, u32) {
    let semantic = semantic.to_ascii_uppercase();
    let suffix = semantic
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .len();
    let index = if suffix == semantic.len() {
        0
    } else {
        semantic[suffix..].parse().unwrap_or(u32::MAX)
    };
    (semantic[..suffix].into(), index)
}
