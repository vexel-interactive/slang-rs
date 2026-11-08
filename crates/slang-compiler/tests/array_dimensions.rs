use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "dimensions.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn dimensions_fold_in_declarations_aliases_and_constructors() {
    let compiled = compile(
        "typedef float Row[Count];static const int Count=Next+1;static const int Next=2;ConstantBuffer<Row> input;float f(){const int Count=2;float[Count+1] a=float[3.5](1,2,3);float b[true][0x3]={a};float c[3>2?3:1]=float[6>>1](4,5,6);return b[0][2]+c[2]+input[2];}",
    );
    assert_eq!(
        compiled.reflection.globals[2].type_name,
        "ConstantBuffer<float[3]>"
    );
    assert_eq!(compiled.reflection.uniform_types["float[3]"].size, 36);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float a[3]"));
    assert!(code.contains("float b[1][3]"));
    assert!(code.contains("float c[3]"));
}

#[test]
fn imported_dimensions_use_their_declaration_scope() {
    let compiled = compile_source(
        "main.slang",
        "import library;ConstantBuffer<Values> input;float f(){const int Count=2;float a[Count]=float[Count](1,2);return a[1]+input[2];}",
        &CompileOptions::default(),
        &MemoryIncludes { files: [("library.slang".into(), "module library;static const int Count=Next+1;static const int Next=2;public typedef float Values[Count];".into())].into() },
    ).unwrap();
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|g| g.name == "input")
            .unwrap()
            .type_name,
        "ConstantBuffer<float[3]>"
    );
}

#[test]
fn empty_values_and_resources_consume_no_bindings() {
    let compiled = compile(
        "struct P{float values[0];Texture2D<float4> textures[0];};P value;ConstantBuffer<P> empty;Texture2D<float4> textures[0];SamplerState samplers[0];ConstantBuffer<float> buffers[65536][65536][0];ConstantBuffer<float> input;",
    );
    assert!(compiled.reflection.global_uniform_binding.is_none());
    assert_eq!(compiled.reflection.global_uniform_size, Some(0));
    assert_eq!(compiled.reflection.uniform_types["P"].size, 0);
    for global in &compiled.reflection.globals[..5] {
        assert!(global.binding.is_none(), "{}", global.name);
    }
    assert_eq!(
        compiled.reflection.globals[5]
            .binding
            .unwrap()
            .register
            .index,
        0
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("[0]"));
    assert!(!code.contains("cbuffer _slang_globals"));
}

#[test]
fn empty_fields_keep_neighbor_offsets_and_source_types() {
    let compiled = compile(
        "struct P{float first;row_major float2x3 skipped[0];float second;float nested[3][0];float third;};ConstantBuffer<P> input;",
    );
    let layout = &compiled.reflection.uniform_types["P"];
    assert_eq!(layout.size, 12);
    assert_eq!(
        layout.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 4, 4, 8, 8]
    );
    assert_eq!(layout.fields[1].type_name, "row_major float2x3[0]");
    assert_eq!(compiled.reflection.uniform_types["float[3][0]"].size, 0);
    assert!(!compiled.artifact.unwrap().contains("[0]"));
    // No element layout needs to be expanded for a zero outer dimension.
    compile("float input[0][4294967295][4294967295][4294967295];");
}

#[test]
fn empty_constructors_copies_defaults_and_overloads_remain_valid() {
    let compiled = compile(
        "typedef float E[0];typedef int I[0];E copy(E x){return x;}float choose(E x){return 1;}float choose(I x){return 2;}float f(){E a=float[0]();E b=copy(a);float c[2][0]=float[0][2]();I d=int[0]();return choose(b)+choose(d);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("[0]"));
    assert!(code.contains("_slang_empty_array_type_0"));
    assert!(code.contains("_slang_empty_array_type_1"));
    compile("struct P{Texture2D<float4> t[0];float x;};float f(){P p=P({},1);return p.x;}");
    compile(
        "typedef float E[3][0];E copy(E x){return x;}float f(){E x=copy(E());float a[0]=x[1];return 1;}",
    );
    compile("Texture2D<float4> t[3][0];float f(){Texture2D<float4> empty[0]=t[1];return 1;}");
}

#[test]
fn dynamic_empty_numeric_locations_evaluate_operands_once() {
    let compiled = compile(
        "typedef float E[0];E array(inout int i){i++;return E();}int index(inout int i){i++;return i;}void write(out float x){x=4;}float f(){int i=0;float v=array(i)[index(i)];E a=E();write(a[index(i)]);a[index(i)]+=2;return i+v;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("[0]"));
    assert!(code.contains("scratch_0"));
    assert_eq!(code.matches("array(i)").count(), 1);
    assert_eq!(code.matches("index(i)").count(), 3);
    assert!(compiled.reflection.globals.is_empty());
    assert!(compiled.reflection.global_uniform_binding.is_none());
}

#[test]
fn private_empty_resource_initialization_keeps_calls_and_mutability() {
    let compiled = compile(
        "typedef Texture2D<float4> E[0];static int counter=0;E make(){counter++;return E();}static E value=make();float f(){E x=E();value=x;return counter;}",
    );
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("make()").count(), 3);
    assert!(
        compiled
            .reflection
            .globals
            .iter()
            .all(|g| g.binding.is_none())
    );
}

#[test]
fn invalid_dimensions_and_empty_initializers_report_diagnostics() {
    for (source, code) in [
        ("int Count;float input[Count];", "T002"),
        ("void f(int Count){float a[Count];}", "T002"),
        ("float input[-1];", "T002"),
        ("float input[1/0];", "T002"),
        (
            "static const int A=B;static const int B=A;float input[A];",
            "T026",
        ),
        ("void f(){float a[0]=float[0](1);}", "T021"),
        ("void f(){float a[0];a[0u]=1;}", "T013"),
        ("void f(){float a[0];a[-1]=1;}", "T013"),
        ("void input[0];", "T002"),
        ("Texture2D<float4> input[0]:register(u0);", "T022"),
        (
            "Texture2D<float4> input[0];float f(int i){return input[i][int2(0)].x;}",
            "I006",
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
