use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "casts.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn numeric_casts_keep_explicit_shapes_and_boolean_conversions() {
    let compiled = compile(include_str!("../../../examples/numeric-casts.slang"));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("int3 integers = ((int3)float3("));
    assert!(code.contains("bool3 enabled = ((bool3)integers);"));
    assert!(code.contains("float2x2 topLeft = ((float2x2)source);"));
    compile("int2x3 run(){return (int2x3)float2x3(1.25);}");
    compile("float3 run(){return (float3)true;}");
}

#[test]
fn casts_keep_input_evaluation_and_value_comparisons() {
    let compiled = compile(
        "float3x3 next(){return float3x3(2);}float run(float vector){float2x2 value=(float2x2)next();return value[1][1]+(vector<3 ? 1.0 : 2.0);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float2x2 value = ((float2x2)next());"));
    assert_eq!(code.matches("((float2x2)next())").count(), 1);
    compile("bool run(float matrix){return matrix<3 && matrix>1;}");
    compile("float3 run(){return (vector<float,3>)vector<float,3>(1);}");
}

#[test]
fn imported_component_types_work_in_template_casts() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public typealias Element=float;".into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "main.slang",
        "import lib;float3 run(int3 input){return (vector<Element,3>)input;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("return ((float3)input);")
    );
}

#[test]
fn incompatible_cast_shapes_are_rejected() {
    for source in [
        "float2 run(){return (float2)float3(1);}",
        "float3 run(){return (float3)float2(1);}",
        "float run(){return (float)float3(1);}",
        "float3x2 run(){return (float3x2)float2x3(1);}",
        "float3x3 run(){return (float3x3)float2x2(1);}",
        "float3 run(){return (float3)float3x3(1);}",
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T008"),
            "{source}: {error}"
        );
    }
}
