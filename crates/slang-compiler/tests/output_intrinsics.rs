use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "output-intrinsics.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn output_intrinsics_accept_writable_fields_swizzles_arrays_and_resources() {
    let compiled = compile(
        "RWStructuredBuffer<float2> integral;struct P{int2 exponent;};float2 run(float2 x){float4 s;float4 c;sincos(x,s.xy,c.zw);float2 fraction=modf(x,integral[0]);P p;float2 mantissa=frexp(x,p.exponent);return fraction+mantissa+s.xy+c.zw;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("sincos(x, (s).xy, (c).zw)"));
    assert!(code.contains("modf(x, (integral)[0])"));
    assert!(code.contains("frexp(x, (p).exponent)"));
    compile(
        "float run(){float integral[2];float fraction=modf(1.2,integral[1]);return fraction+integral[1];}",
    );
}

#[test]
fn output_shapes_and_types_determine_explicit_input_conversions() {
    let compiled = compile(
        "float2 run(){float2 s;float2 c;sincos(1,s,c);int2 exponent;float2 mantissa=frexp(8,exponent);double integral;double fraction=modf(1.2,integral);return mantissa+s+c+float(fraction);}",
    );
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Expression(value) = &body[2].kind else {
        panic!()
    };
    let ExprKind::Call { arguments, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(arguments[0].ty.to_string(), "float2");
    assert!(matches!(arguments[0].kind, ExprKind::Cast { .. }));
    let StmtKind::Variable(mantissa) = &body[4].kind else {
        panic!()
    };
    assert_eq!(
        mantissa.initializer.as_ref().unwrap().ty.to_string(),
        "float2"
    );
    compile(
        "float2 run(){float2 s;float2 c;sincos({1.0,2.0},s,c);float2 integral;return modf({1.2,2.3},integral)+s+c;}",
    );
}

#[test]
fn modf_matrices_and_user_functions_keep_their_selected_signatures() {
    compile("float2x2 run(){float2x2 integral;return modf(float2x2(1.2),integral);}");
    compile(
        "void sincos(int x,out int s,out int c){s=x;c=x;}int run(){int s;int c;sincos(1,s,c);return s+c;}",
    );
    compile(
        "half run(half x){half s;half c;sincos(x,s,c);half integral;return modf(x,integral)+s+c;}",
    );
}

#[test]
fn output_arguments_require_exact_types_and_distinct_writable_locations() {
    for (source, code) in [
        ("void run(){float c;sincos(1.0,2.0,c);}", "T015"),
        (
            "void run(){float2 s;float2 c;sincos(float2(1),s.xx,c);}",
            "T015",
        ),
        (
            "void run(){const float s=1;float c;sincos(1.0,s,c);}",
            "T015",
        ),
        ("void run(){int s;float c;sincos(1.0,s,c);}", "T014"),
        ("void run(){float s;float c;sincos(float2(1),s,c);}", "T014"),
        (
            "float run(){float exponent;return frexp(1.0,exponent);}",
            "T014",
        ),
        (
            "float2 run(){int exponent;return frexp(float2(1),exponent);}",
            "T014",
        ),
        ("void run(){float s;float c;sincos(true,s,c);}", "T014"),
        ("void run(){float s;sincos(1.0,s);}", "T014"),
        (
            "void run(){float2x2 s;float2x2 c;sincos(float2x2(1),s,c);}",
            "T014",
        ),
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == code),
            "{source}: {error}"
        );
    }
}
