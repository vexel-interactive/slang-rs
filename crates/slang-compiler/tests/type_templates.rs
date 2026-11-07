use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "templates.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn numeric_templates_share_canonical_types_and_constructor_lowering() {
    let compiled = compile(include_str!(
        "../../../examples/numeric-type-templates.slang"
    ));
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "StructuredBuffer<float3>"
    );
    assert_eq!(
        compiled.reflection.globals[1].type_name,
        "RWStructuredBuffer<float3>"
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float3x3 basis = float3x3(float3("));
    assert!(code.contains("float3 bias = ((float3)0.5);"));
    assert!(!code.contains("vector<"));
    assert!(!code.contains("matrix<"));
    compile(
        "float3 choose(float3 value){return value;}float3 run(){return choose(vector<float,3>(1));}",
    );
}

#[test]
fn default_arguments_nested_resources_and_array_constructors_keep_types() {
    let compiled = compile(
        "static vector first=vector<>(1);static vector<> second=vector<>(2);static vector<double> third=vector<double>(3);static matrix fourth=matrix<>(4);static matrix<float,2> fifth=matrix<float,2>(5);StructuredBuffer<vector<int,0x2>> input;float run(){vector<float,2> values[2]=vector<float,2>[2](vector<float,2>(1),vector<float,2>(2));return values[1].x;}",
    );
    let types: Vec<_> = compiled
        .ir
        .globals
        .iter()
        .map(|g| g.ty.to_string())
        .collect();
    assert_eq!(
        types,
        [
            "float4",
            "float4",
            "double4",
            "float4x4",
            "float2x4",
            "StructuredBuffer<int2>"
        ]
    );
    compile("float run(){vector<float,2>(1,2);return vector<float,2>(3,4).x;}");
}

#[test]
fn imported_scalar_aliases_resolve_inside_numeric_type_arguments() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;typealias Element=float;public typealias Value=vector<Element,3>;public Value read(){return vector<Element,3>(1,2,3);}".into())].into() };
    let compiled = compile_source(
        "main.slang",
        "import lib;float3 run(){return read()+Value(1);}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("return float3(1, 2, 3);")
    );
}

#[test]
fn template_spelling_is_still_available_for_value_bindings() {
    let compiled = compile(
        "float matrix(float value){return value+1;}float run(float vector){return matrix(vector);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _slang_matrix(float value)"));
    assert!(code.contains("return _slang_matrix(_slang_vector);"));
}

#[test]
fn invalid_numeric_type_arguments_fail_before_lowering() {
    for source in [
        "float run(){return vector(1).x;}",
        "float run(){return matrix(1)[0][0];}",
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "T014"));
    }
    for source in [
        "vector<float,0> value;",
        "vector<float,5> value;",
        "vector<float,2,3> value;",
        "matrix<float,2,3,4> value;",
        "vector<float2,3> value;",
        "vector<3,2> value;",
        "vector<float,float> value;",
        "StructuredBuffer<3> value;",
        "Texture2D<3> value;",
        "ConstantBuffer<3> value;",
        "float run(){return vector<float,0>(1).x;}",
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T002"),
            "{source}: {error}"
        );
    }
}
