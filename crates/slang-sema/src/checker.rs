use crate::Type;
use serde::Serialize;
use slang_source::{Diagnostic, Span};
use slang_syntax::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct EntryPoint {
    pub name: String,
    pub stage: ShaderStage,
    /// Present only for compute shaders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_group_size: Option<[u32; 3]>,
}

/// Shader execution models currently checked by the native frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShaderStage {
    Compute,
    Vertex,
    Fragment,
}

#[derive(Clone, Debug)]
pub struct CheckedModule {
    pub module: Module,
    pub expression_types: BTreeMap<ExprId, Type>,
    pub declaration_types: BTreeMap<SpanKey, Type>,
    pub entry_points: Vec<EntryPoint>,
}

/// Byte-span key used to distinguish declarations in separate scopes and files.
pub type SpanKey = (usize, usize, usize);
pub fn span_key(span: Span) -> SpanKey {
    (span.source.0, span.start, span.end)
}

#[derive(Clone)]
pub(crate) struct Binding {
    pub ty: Type,
    pub writable: bool,
}

#[derive(Clone)]
pub(crate) struct Signature {
    pub parameters: Vec<Type>,
    pub output_parameters: Vec<bool>,
    pub result: Type,
    pub defined: bool,
}

pub(crate) struct Checker {
    pub scopes: Vec<BTreeMap<String, Binding>>,
    pub functions: BTreeMap<String, Vec<Signature>>,
    pub structures: BTreeMap<String, BTreeMap<String, Type>>,
    pub expression_types: BTreeMap<ExprId, Type>,
    pub declaration_types: BTreeMap<SpanKey, Type>,
    pub diagnostics: Vec<Diagnostic>,
    pub return_type: Type,
    pub loops: usize,
}

pub fn check(module: Module) -> Result<CheckedModule, Vec<Diagnostic>> {
    let mut checker = Checker {
        scopes: vec![BTreeMap::new()],
        functions: BTreeMap::new(),
        structures: BTreeMap::new(),
        expression_types: BTreeMap::new(),
        declaration_types: BTreeMap::new(),
        diagnostics: vec![],
        return_type: Type::Void,
        loops: 0,
    };
    checker.declare(&module);
    let mut entry_points = vec![];
    for declaration in &module.declarations {
        match declaration {
            Declaration::Global(variable) => checker.initializer(variable),
            Declaration::Function(function) => {
                if let Some(entry) = checker.entry_point(function, &module) {
                    entry_points.push(entry);
                }
                checker.function(function);
            }
            Declaration::Struct(structure) => {
                for field in &structure.fields {
                    if field.initializer.is_some()
                        || field.modifiers.iter().any(|m| m != "nointerpolation")
                    {
                        checker.error(
                            "T001",
                            field.span,
                            "struct field initializers and modifiers other than nointerpolation are not implemented",
                        );
                    }
                    checker.check_interpolation(&field.modifiers, field.span);
                }
            }
        }
    }
    if checker.diagnostics.is_empty() {
        Ok(CheckedModule {
            module,
            expression_types: checker.expression_types,
            declaration_types: checker.declaration_types,
            entry_points,
        })
    } else {
        Err(checker.diagnostics)
    }
}

impl Checker {
    pub fn error(&mut self, code: &str, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(code, span, message));
    }

    pub fn require_conversion(&mut self, actual: &Type, expected: &Type, span: Span) {
        if !actual.convertible_to(expected) {
            self.error(
                "T008",
                span,
                format!("cannot convert {actual} to {expected}"),
            );
        }
    }

    pub fn condition(&mut self, expression: &Expr) {
        let ty = self.expression(expression);
        if !matches!(ty, Type::Scalar(_) | Type::Error) {
            self.error("T009", expression.span, "condition must have a scalar type");
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slang_source::SourceId;
    fn run(text: &str) -> Result<CheckedModule, Vec<Diagnostic>> {
        check(slang_syntax::parse(&slang_lexer::lex(SourceId(0), text).tokens).unwrap())
    }

    #[test]
    fn compute_resource_access_and_overloads() {
        let text = "float twice(float x){return x*2.0;} int twice(int x){return x*2;} RWStructuredBuffer<float> output; [shader(\"compute\")][numthreads(64,1,1)] void main(uint3 id:SV_DispatchThreadID){output[id.x]=twice(float(id.x));}";
        let module = run(text).unwrap();
        assert_eq!(module.entry_points[0].thread_group_size, Some([64, 1, 1]));
    }

    #[test]
    fn invalid_programs_are_rejected() {
        for text in [
            "void f(){missing=1;}",
            "void f(){const int x=1;x=2;}",
            "void f(){break;}",
            "int f(){if(true)return 1;}",
            "StructuredBuffer<float> b; void f(){b[0]=1;}",
            "void f(){float2 x;x.z=1;}",
            "void f(){float2 x;x.xx=float2(1,2);}",
            "[shader(\"compute\")][numthreads(0,1,1)]void main(){}",
            "void f(){int x;{int y;} x=y;}",
            "struct A { A x; };",
            "void f(out int a){} void g(){f(1);}",
        ] {
            assert!(run(text).is_err(), "{text}");
        }
    }

    #[test]
    fn globals_and_parameter_qualifiers() {
        assert!(run("static float x=1.0; void f(){x=2.0;}").is_ok());
        assert!(run("float x; void f(){x=2.0;}").is_err());
        assert!(run("void f(){groupshared int x;}").is_err());
        assert!(run("void f(const out int x){}").is_err());
        assert!(run("void f(in out int x){}").is_err());
        assert!(
            slang_syntax::parse(&slang_lexer::lex(SourceId(0), "void f(){int return;}").tokens)
                .is_err()
        );
    }
}
