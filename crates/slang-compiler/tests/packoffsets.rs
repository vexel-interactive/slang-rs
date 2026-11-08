use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "packed.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn explicit_fields_precede_automatic_packing_and_preserve_overlaps() {
    let compiled = compile(
        "cbuffer P{float a:packoffset(c2.y);float b;float2 v:packoffset(c000.z);float alias:packoffset(c2.y);};",
    );
    let buffer = &compiled.reflection.globals[0];
    let layout =
        &compiled.reflection.uniform_types[&buffer.uniform_layout.as_ref().unwrap().type_name];
    assert_eq!(layout.size, 44);
    assert_eq!(
        layout
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.offset))
            .collect::<Vec<_>>(),
        [("a", 36), ("b", 40), ("v", 8), ("alias", 36)]
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float a : packoffset(c2.y);"));
    assert!(code.contains("float2 v : packoffset(c0.z);"));
    assert!(!code[..code.find("cbuffer ").unwrap()].contains("packoffset"));
}

#[test]
fn packing_is_relative_to_each_buffer_and_static_members_have_no_offset() {
    let compiled = compile(
        "cbuffer P{double wide:packoffset(c0.y);float tail;static float storage:packoffset(c7)=1;};cbuffer Q{float values[2]:packoffset(c2);float after;};",
    );
    let layouts: Vec<_> = compiled
        .reflection
        .globals
        .iter()
        .map(|g| &compiled.reflection.uniform_types[&g.uniform_layout.as_ref().unwrap().type_name])
        .collect();
    assert_eq!(layouts[0].size, 16);
    assert_eq!(
        layouts[0]
            .fields
            .iter()
            .map(|f| f.offset)
            .collect::<Vec<_>>(),
        [4, 12]
    );
    assert_eq!(layouts[1].size, 56);
    assert_eq!(
        layouts[1]
            .fields
            .iter()
            .map(|f| f.offset)
            .collect::<Vec<_>>(),
        [32, 52]
    );
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("static float storage = 1;")
    );
}

#[test]
fn imported_packing_and_default_arguments_keep_field_identity() {
    let includes = MemoryIncludes{files:[("library.slang".into(),"module library;public cbuffer P{public float value:packoffset(c3.w);public float after;};public float f(float input=value){return input+after;}".into())].into()};
    let compiled = compile_source(
        "root.slang",
        "import library;float g(){return f();}",
        &CompileOptions {
            check_only: true,
            ..CompileOptions::default()
        },
        &includes,
    )
    .unwrap();
    let layout = &compiled.reflection.uniform_types[&compiled.reflection.globals[0]
        .uniform_layout
        .as_ref()
        .unwrap()
        .type_name];
    assert_eq!(layout.size, 68);
    assert_eq!(
        layout.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [60, 64]
    );
    assert!(compiled.artifact.is_none());
}

#[test]
fn malformed_and_unsupported_packing_is_diagnosed() {
    for source in [
        "cbuffer P{float value:packoffset(b0);};",
        "cbuffer P{float value:packoffset(c0.q);};",
        "cbuffer P{float value:packoffset(c);};",
        "cbuffer P{float value:packoffset(c2147483648);};",
        "cbuffer P{float value:packoffset(c0):packoffset(c1);};",
        "cbuffer P:packoffset(c0){float value;};",
        "float value:packoffset(c0);",
        "void f(){float value:packoffset(c0)=1;}",
        "void f(float value:packoffset(c0)){}",
        "struct S{float value:packoffset(c0);};",
    ] {
        assert!(
            compile_source(
                "invalid.slang",
                source,
                &CompileOptions::default(),
                &MemoryIncludes::default()
            )
            .is_err(),
            "{source}"
        );
    }
}
