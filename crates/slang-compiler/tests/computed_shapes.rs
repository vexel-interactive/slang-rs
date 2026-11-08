use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "shapes.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn expressions_resolve_in_aliases_resources_casts_and_direct_constructors() {
    let compiled = compile(
        "typedef vector<float,1+2> V;typedef row_major matrix<float,2,(1<<1)+1> M;ConstantBuffer<V> vectorValue;ConstantBuffer<M> matrixValue;float f(){V v=vector<float,1+2>(1,2,3);M m=(matrix<float,2,(1<<1)+1>)matrixValue;return v.z+m[1][2]+vectorValue.x;}",
    );
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "ConstantBuffer<float3>"
    );
    assert_eq!(
        compiled.reflection.globals[1].type_name,
        "ConstantBuffer<row_major float2x3>"
    );
    assert_eq!(
        compiled.reflection.uniform_types["row_major float2x3"].size,
        28
    );
    compile("float f(){vector<float,1<2?3:2> value=vector<float,1<2?3:2>(1,2,3);return value.z;}");
}

#[test]
fn shape_conversions_truncate_while_identity_and_arithmetic_keep_intermediates() {
    let compiled = compile(
        "static const uint16_t Explicit=uint16_t(65538);static const uint16_t Implicit=65538;static const int16_t Signed=65538;vector<float,Explicit> a;vector<float,Implicit> b;matrix<int16_t,Signed,uint16_t(65539)> c;static const uint16_t Wide=uint16_t(65535)+uint16_t(3);vector<float,Wide==2?3:2> d;float f(){const uint16_t Local=65538;vector<float,Local> v=1;return v[1];}",
    );
    let types = compiled
        .reflection
        .globals
        .iter()
        .map(|g| (g.name.as_str(), g.type_name.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(types["a"], "float2");
    assert_eq!(types["b"], "float2");
    assert_eq!(types["c"], "int16_t2x3");
    assert_eq!(types["d"], "float2");
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float2 v"), "{code}");
    let compiled = compile(
        "vector<float,uint16_t(uint16_t(65535)+uint16_t(3))==2?3:2> identity;vector<float,uint16_t(int(uint16_t(65535)+uint16_t(3)))==2?3:2> converted;",
    );
    assert_eq!(compiled.reflection.globals[0].type_name, "float2");
    assert_eq!(compiled.reflection.globals[1].type_name, "float3");
}

#[test]
fn globals_use_forward_typed_constants_and_locals_keep_lexical_priority() {
    let compiled = compile(
        "typedef vector<float,Count> V;static const uint Count=Next+1u;static const uint Next=~0u+3u;V input;float f(){const int Count=2;vector<float,Count> local=1;return local.y+input.z;}",
    );
    assert_eq!(compiled.reflection.globals[2].type_name, "float3");
    assert!(compiled.artifact.unwrap().contains("float2 local"));
    let compiled =
        compile("static const uint Count=(0xffffffffu+1u)==0u?3u:2u;vector<float,Count> input;");
    assert_eq!(compiled.reflection.globals[1].type_name, "float2");
    compile(
        "float f(){const uint Count=(0xffffffffu+1u)==0u?3u:2u;vector<float,Count> value=1;return value.y;}",
    );
}

#[test]
fn scalar_conversions_and_supported_comparisons_form_dimensions() {
    for shape in [
        "true",
        "!false",
        "bool(3)",
        "int(3.5)",
        "3.5",
        "(1<2?3:2)",
        "(3==2?2:3)",
        "int(2<=3)+2",
    ] {
        let compiled = compile(&format!(
            "vector<float,{shape}> input;float f(){{return input[0];}}"
        ));
        let expected = if matches!(shape, "true" | "!false" | "bool(3)") {
            "float1"
        } else {
            "float3"
        };
        assert_eq!(
            compiled.reflection.globals[0].type_name, expected,
            "{shape}"
        );
    }
}

#[test]
fn imported_shape_constants_and_aliases_resolve_in_their_declaration_module() {
    let compiled = compile_source(
        "main.slang",
        "import library;ConstantBuffer<Value> input;float f(){const int Count=1;return input.z;}",
        &CompileOptions::default(),
        &MemoryIncludes { files: [("library.slang".into(), "module library;static const int Count=Next+1;static const int Next=2;public typealias Value=vector<float,Count>;".into())].into() },
    ).unwrap();
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|g| g.name == "input")
            .unwrap()
            .type_name,
        "ConstantBuffer<float3>"
    );
}

#[test]
fn runtime_dimensions_and_dependency_cycles_fail_with_bounded_diagnostics() {
    for (source, code) in [
        ("int Count;vector<float,Count> input;", "T002"),
        ("void f(int Count){vector<float,Count> value;}", "T002"),
        (
            "static const int A=B;static const int B=A;vector<float,A> value;",
            "T026",
        ),
        ("vector<float,1/0> value;", "T002"),
        ("vector<float,4+1> value;", "T002"),
    ] {
        let error = compile_source(
            "invalid.slang",
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
    let mut source = String::new();
    for i in 0..64 {
        source.push_str(&format!("static const int C{i}=C{};", i + 1));
    }
    source.push_str("static const int C64=3;vector<float,C0> value;");
    let error = compile_source(
        "deep.slang",
        &source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T026"));
}

#[test]
fn right_angle_operators_follow_upstream_template_argument_grammar() {
    for shape in ["(8>>1)", "(3>2)", "(3>=2)"] {
        let error = compile_source(
            "angles.slang",
            &format!("vector<float,{shape}> value;"),
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "S001"));
    }
    compile("static const int Count=(8>>1)-1;vector<float,Count> input;");
    compile("static const int Count=((1<<40)>>40)+2;vector<float,Count> input;");
}
