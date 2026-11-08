use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "cbuffer.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn direct_cbuffer_fields_are_read_only_globals_with_one_reflected_resource() {
    let compiled = compile(
        "cbuffer Params:register(b2,space1){float scale;float2 values[2];};float f(){return scale+values[1].x;}",
    );
    assert_eq!(compiled.reflection.globals.len(), 1);
    let buffer = &compiled.reflection.globals[0];
    assert_eq!(buffer.name, "Params");
    let binding = buffer.binding.unwrap();
    assert_eq!(
        (
            binding.register.index,
            binding.register.space,
            binding.count
        ),
        (2, 1, 1)
    );
    assert!(binding.explicit);
    assert!(compiled.reflection.global_uniform_binding.is_none());
    let layout =
        &compiled.reflection.uniform_types[&buffer.uniform_layout.as_ref().unwrap().type_name];
    assert_eq!(layout.size, 40);
    assert_eq!(
        layout
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.offset))
            .collect::<Vec<_>>(),
        [("scale", 0), ("values", 16)]
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains(&format!(
        "cbuffer {} : register(b2, space1)",
        buffer.target_name
    )));
    assert_eq!(code.matches("cbuffer ").count(), 1);
    assert!(code.contains("return (scale + ((values)[1]).x)"));
}

#[test]
fn buffer_names_have_their_own_namespace_and_fields_keep_lexical_shadowing() {
    let compiled = compile(
        "cbuffer Params{float scale;}float Params;float f(float scale){return scale+Params;}float g(){float scale=scale;return scale;}",
    );
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .filter(|g| g.name == "Params")
            .count(),
        2
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("cbuffer "));
    assert!(code.contains("float Params;"));
    assert!(code.contains(" = scale;"));
    for source in [
        "cbuffer Params{float scale;};float f(){return Params;}",
        "cbuffer Params{float scale;};float f(){return Params.scale;}",
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "T011"));
    }
}

#[test]
fn field_initializers_are_checked_and_static_members_use_private_storage() {
    let compiled = compile(
        "cbuffer Params{float scale=2;const int count=4;static float counter=1;};float f(float value=scale){counter=counter+1;return value+float(count)+counter;}float g(){return f();}",
    );
    assert_eq!(compiled.reflection.globals.len(), 1);
    let layout = &compiled.reflection.uniform_types[&compiled.reflection.globals[0]
        .uniform_layout
        .as_ref()
        .unwrap()
        .type_name];
    assert_eq!(
        layout
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["scale", "count"]
    );
    assert_eq!(layout.size, 8);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("static float counter = 1;"));
    assert!(code.contains("return scale;"));
    let begin = code.find("cbuffer ").unwrap();
    let end = begin + code[begin..].find("};").unwrap();
    assert!(!code[begin..end].contains(" = "));
}

#[test]
fn imported_cbuffer_fields_preserve_access_and_module_members() {
    let includes=MemoryIncludes {files:[
        ("library.slang".into(),"module library;public cbuffer Params:register(b3){public float scale;float hidden;};__include member;".into()),
        ("member.slang".into(),"implementing library;public cbuffer Other{public float offset;};".into()),
    ].into()};
    let compiled = compile_source(
        "root.slang",
        "import library;float f(){return scale+offset;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert_eq!(compiled.reflection.globals.len(), 2);
    assert!(
        compiled
            .reflection
            .globals
            .iter()
            .all(|g| g.module.as_deref() == Some("library"))
    );
    assert_eq!(compiled.ir.constant_buffers.len(), 2);
    assert!(compiled.reflection.global_uniform_binding.is_none());
    let error = compile_source(
        "root.slang",
        "import library;float f(){return hidden;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "T011" | "T018"))
    );
}

#[test]
fn write_locations_wrong_bindings_and_resource_fields_are_rejected() {
    for source in [
        "cbuffer Params{float scale;};void f(){scale=1;}",
        "cbuffer Params{float2 values[2];};void f(){values[0].x=1;}",
        "void set(out float value){value=1;}cbuffer Params{float scale;};void f(){set(scale);}",
        "cbuffer Params:register(t0){float scale;};",
        "cbuffer Params{Texture2D<float> image;};",
        "cbuffer Params{float scale;};cbuffer Other{float scale;};float f(){return scale;}",
        "cbuffer Params{float scale;};cbuffer Params{float offset;};",
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
fn empty_constant_buffers_keep_declarations_without_consuming_registers() {
    let compiled = compile(
        "struct Empty{};cbuffer First:register(b3){};cbuffer Second{static float storage=1;Empty empty;};ConstantBuffer<Empty> third;cbuffer Full{float value;};",
    );
    assert_eq!(compiled.reflection.globals.len(), 4);
    for buffer in &compiled.reflection.globals[..3] {
        assert!(buffer.binding.is_none());
        let name = &buffer.uniform_layout.as_ref().unwrap().type_name;
        assert_eq!(compiled.reflection.uniform_types[name].size, 0);
    }
    assert_eq!(
        compiled.reflection.globals[3]
            .binding
            .unwrap()
            .register
            .index,
        0
    );
    assert_eq!(compiled.artifact.unwrap().matches("cbuffer ").count(), 1);
}

#[test]
fn constant_buffer_registers_reserve_only_handles_and_check_only_keeps_layout() {
    let compiled = compile_source(
        "cbuffer.slang",
        "cbuffer Fixed:register(b0){float x;};cbuffer Automatic{float y;};uniform float root;",
        &CompileOptions {
            check_only: true,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(compiled.artifact.is_none());
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .map(|g| (g.name.as_str(), g.binding.map(|b| b.register.index)))
            .collect::<Vec<_>>(),
        [("Fixed", Some(0)), ("Automatic", Some(2)), ("root", None)]
    );
    assert_eq!(
        compiled
            .reflection
            .global_uniform_binding
            .unwrap()
            .register
            .index,
        1
    );
    assert_eq!(compiled.reflection.global_uniform_size, Some(4));
}
