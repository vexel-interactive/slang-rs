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

#[test]
fn packed_struct_handles_lower_to_loaded_values_with_source_reflection() {
    let compiled = compile(
        "struct S{float a:packoffset(c2.y);float b;float values[2];};ConstantBuffer<S> params:register(b3);float f(ConstantBuffer<S> p){return p.a+p.b+p.values[1];}float g(){return f(params);}",
    );
    let buffer = &compiled.reflection.globals[0];
    assert_eq!(buffer.type_name, "ConstantBuffer<S>");
    assert_eq!(buffer.resource_access.as_deref(), Some("read"));
    assert_eq!(buffer.binding.unwrap().register.index, 3);
    assert_eq!(compiled.reflection.uniform_types["S"].size, 68);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float f(S p)"));
    assert!(!code.contains("static const S params"));
    assert!(code.contains("packoffset(c2.y)"));
    assert!(code.contains("return _result;"));
}

#[test]
fn nested_packed_uniform_arrays_keep_absolute_offsets_and_bounded_expansion() {
    let compiled = compile(
        "struct S{float a:packoffset(c2.y);float b;};float x;uniform S data[2];float y;float f(){return x+data[1].a+y;}",
    );
    assert_eq!(compiled.reflection.global_uniform_size, Some(112));
    let data = &compiled.reflection.globals[1];
    assert_eq!(data.type_name, "S[2]");
    assert_eq!(data.uniform_layout.as_ref().unwrap().offset, 16);
    let code = compiled.artifact.unwrap();
    assert!(code.contains(&format!("{}()", data.target_name)));
    assert!(code.contains("packoffset(c6.y)"));
    for source in [
        "struct S{float a:packoffset(c2);};S data[65537];",
        "struct S{float a:packoffset(c2);};ConstantBuffer<S> params[2];",
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "I004"));
    }
}

#[test]
fn imported_packed_templates_keep_original_names_and_field_access_rules() {
    let includes=MemoryIncludes{files:[("library.slang".into(),"module library;public struct S{public float a:packoffset(c2.y);public float b;};public ConstantBuffer<S> params:register(b3);public float f(float value=params.a){return value+params.b;}".into())].into()};
    let compiled = compile_source(
        "root.slang",
        "import library;float g(){return f();}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert_eq!(compiled.reflection.globals[0].name, "params");
    assert_eq!(
        compiled.reflection.globals[0].module.as_deref(),
        Some("library")
    );
    let error = compile_source(
        "root.slang",
        "import library;void g(){params.a=1;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T015"));
}

#[test]
fn packed_global_loads_do_not_capture_local_bindings_without_module_linking() {
    let source = "struct S{float a:packoffset(c2);float b;};S data;float f(){S data=S(1,2);return data.a;}float g(){return data.a;}";
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("root.slang", source);
    let tokens = slang_preprocessor::preprocess(
        &mut sources,
        root,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .tokens;
    let syntax = slang_syntax::parse(&tokens).unwrap();
    let checked = slang_sema::check(syntax).unwrap();
    let ir = slang_ir::lower(&checked).unwrap();
    let code = slang_codegen::emit(&ir, slang_codegen::Target::Hlsl).unwrap();
    assert!(code.contains("return (data).a;"));
    assert!(code.contains("uniform_view_load_"));
}

#[test]
fn packing_empty_values_does_not_create_storage_or_consume_a_register() {
    let compiled = compile(
        "struct E{};struct S{E empty:packoffset(c4);};ConstantBuffer<S> empty;cbuffer Full{float value;};",
    );
    assert!(compiled.reflection.globals[0].binding.is_none());
    assert_eq!(compiled.reflection.uniform_types["S"].size, 0);
    assert_eq!(
        compiled.reflection.globals[1]
            .binding
            .unwrap()
            .register
            .index,
        0
    );
}
