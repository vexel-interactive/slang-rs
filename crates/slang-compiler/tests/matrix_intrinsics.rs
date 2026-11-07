use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "matrix-intrinsics.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn matrix_helpers_reuse_signatures_and_keep_vector_output_types() {
    let compiled = compile(
        "float2x3 next(){return float2x3(1.25);}float run(){float2x3 s;float2x3 c;sincos(next(),s,c);sincos(float2x3(2),s,c);int2x3 exponent;float2x3 mantissa=frexp(s,exponent);float2x3 integral;float2x3 fraction=modf(c,integral);return mantissa[0][0]+fraction[1][2]+float(exponent[0][0]);}",
    );
    let helpers: Vec<_> = compiled
        .ir
        .functions
        .iter()
        .filter(|f| f.name.starts_with("_slang_math_"))
        .collect();
    assert_eq!(helpers.len(), 3);
    let sincos = helpers.iter().find(|f| f.name.contains("sincos")).unwrap();
    let StmtKind::Block(body) = &sincos.body.as_ref().unwrap().kind else {
        panic!()
    };
    assert_eq!(body.len(), 2);
    for row in body {
        let StmtKind::Expression(value) = &row.kind else {
            panic!()
        };
        let ExprKind::IntrinsicCall { name, arguments } = &value.kind else {
            panic!()
        };
        assert_eq!(name, "sincos");
        assert!(arguments.iter().all(|a| a.ty.to_string() == "float3"));
    }
    let frexp = helpers.iter().find(|f| f.name.contains("frexp")).unwrap();
    assert_eq!(frexp.parameters[1].ty.to_string(), "int2x3");
    assert_eq!(frexp.return_type.to_string(), "float2x3");
    assert_eq!(compiled.artifact.unwrap().matches("next()").count(), 3);
}

#[test]
fn matrix_defaults_and_source_prefixes_survive_later_lowering() {
    let compiled = compile(
        "static int2x2 exponent;struct P{float2x2 value=frexp(float2x2(1.25),exponent);};float run(){float _slang_math_frexp_0=2;P p=P();return p.value[0][0]+_slang_math_frexp_0;}",
    );
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.name.starts_with("_slang_math__frexp_"))
    );
    assert_eq!(compiled.reflection.globals.len(), 1);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("frexp((_input)[0u], (_out_0)[0u])"));
    assert!(!code.contains("frexp(((float2x2)"));
}

#[test]
fn user_intrinsic_names_cannot_capture_builtins_in_imported_modules() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public float read(float x){return sin(x);}".into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "main.slang",
        "import lib;float sin(float x){return x+5;}float run(){return sin(1)+read(2);}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return sin(x);"));
    assert!(code.contains("float _slang_sin(float x)"));
    assert!(code.contains("_slang_sin(1)"));
    let compiled = compile("[shader(\"compute\")][numthreads(1,1,1)]void length(){}");
    assert_eq!(compiled.reflection.entry_points[0].name, "length");
    assert_eq!(
        compiled.reflection.entry_points[0].target_name,
        "_slang_length"
    );
}
