use slang_compiler::{
    Compilation, CompileOptions, MemoryIncludes, RegisterClass, Severity, compile_source,
};

fn compile(source: &str) -> Compilation {
    compile_source(
        "registers.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn reject(source: &str, code: &str) {
    let error = compile_source(
        "registers.slang",
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

#[test]
fn every_resource_family_keeps_its_explicit_class_index_and_space() {
    let compiled = compile(
        "struct Params{float value;};
         ConstantBuffer<Params> params:register(b2,space3);
         Texture2D<float4> texture:register(t4,space1);
         RWTexture3D<float> writable:register(u5,space1);
         StructuredBuffer<float> input:register(t6);
         RWStructuredBuffer<float> output:register(u7);
         ByteAddressBuffer bytes:register(t8,space2);
         RWByteAddressBuffer rwBytes:register(u9,space2);
         SamplerState sampler:register(s10);
         SamplerComparisonState comparison:register(s11,space4);",
    );
    assert!(compiled.diagnostics.is_empty());
    let code = compiled.artifact.unwrap();
    for (global, (class, index, space)) in compiled.reflection.globals.iter().zip([
        (RegisterClass::ConstantBuffer, 2, 3),
        (RegisterClass::ShaderResource, 4, 1),
        (RegisterClass::UnorderedAccess, 5, 1),
        (RegisterClass::ShaderResource, 6, 0),
        (RegisterClass::UnorderedAccess, 7, 0),
        (RegisterClass::ShaderResource, 8, 2),
        (RegisterClass::UnorderedAccess, 9, 2),
        (RegisterClass::Sampler, 10, 0),
        (RegisterClass::Sampler, 11, 4),
    ]) {
        let binding = global.binding.unwrap();
        assert_eq!(binding.register.class, class);
        assert_eq!(binding.register.index, index);
        assert_eq!(binding.register.space, space);
        assert_eq!(binding.count, 1);
        assert!(code.contains(&format!("{} : {}", global.target_name, binding.register)));
    }
}

#[test]
fn annotations_belong_to_each_declarator_and_normalize_decimal_spelling() {
    let compiled = compile(
        "Texture2D<float> a:register(t01.x,space02),b:register(t3,space0),c;
         Texture2D<float> d:register(t4):register(t04,space00);",
    );
    let globals = &compiled.reflection.globals;
    assert_eq!(globals[0].binding.unwrap().register.index, 1);
    assert_eq!(globals[0].binding.unwrap().register.space, 2);
    assert_eq!(globals[1].binding.unwrap().register.index, 3);
    assert_eq!(globals[1].binding.unwrap().register.space, 0);
    assert!(!globals[2].binding.unwrap().explicit);
    assert_eq!(globals[2].binding.unwrap().register.index, 0);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("a : register(t1, space2)"));
    assert!(code.contains("b : register(t3)"));
    assert!(code.contains("d : register(t4)"));
    assert!(!code.contains("t01") && !code.contains(".x"));
    reject("Texture2D<float> a:register(t1):register(t2);", "S001");
}

#[test]
fn resource_arrays_reserve_the_product_of_their_dimensions() {
    let compiled = compile(
        "struct Params{float value;};
         typedef Texture2D<float> Image;
         Image images[2][3]:register(t2,space4);
         ConstantBuffer<Params> params[2]:register(b3);
         RWStructuredBuffer<float> outputs[4]:register(u0);
         SamplerState samplers[2]:register(s5);
         float run(){outputs[1][0]=images[1][2][uint2(0)]+params[1].value;
             return outputs[1][0];}",
    );
    assert!(compiled.diagnostics.is_empty());
    for (global, count) in compiled.reflection.globals.iter().zip([6, 2, 4, 2]) {
        assert_eq!(global.binding.unwrap().count, count);
    }
    assert_eq!(
        compiled.reflection.globals[0].resource_access.as_deref(),
        Some("read")
    );
    assert_eq!(
        compiled.reflection.globals[1].resource_access.as_deref(),
        Some("read")
    );
    assert_eq!(
        compiled.reflection.globals[2].resource_access.as_deref(),
        Some("read_write")
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("images[2][3] : register(t2, space4)"));
    assert!(code.contains("params[2] : register(b3)"));
    reject(
        "struct Params{Texture2D<float> texture;};ConstantBuffer<Params> params[2];",
        "T002",
    );
    reject("StructuredBuffer<Texture2D<float>[2]> invalid;", "T002");
    reject(
        "struct Params{float value;};ConstantBuffer<Params> params[2];void f(){params[0].value=1;}",
        "T015",
    );
    reject(
        "Texture2D<float> images[2];void f(){images[0][uint2(0)]=1;}",
        "T015",
    );
}

#[test]
fn overlaps_warn_without_failing_checks_and_keep_nested_ranges_visible() {
    let source = "Texture2D<float> outer[10]:register(t0,space2);
                  Texture2D<float> inner[2]:register(t2,space2),next:register(t8,space2);
                  Texture2D<float> adjacent:register(t10,space2),otherSpace:register(t0,space3);
                  RWTexture2D<float> otherClass:register(u0,space2);";
    let compiled = compile(source);
    assert_eq!(compiled.diagnostics.len(), 2);
    assert!(
        compiled
            .diagnostics
            .iter()
            .all(|d| d.severity == Severity::Warning && d.code == "T023")
    );
    assert!(
        compiled
            .diagnostics
            .iter()
            .all(|d| d.message.contains("outer") && !d.notes.is_empty())
    );
    let checked = compile_source(
        "registers.slang",
        source,
        &CompileOptions {
            check_only: true,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(checked.artifact.is_none());
    assert_eq!(checked.diagnostics, compiled.diagnostics);
    reject(&format!("{source}float f(){{return missing;}}"), "T011");
}

#[test]
fn wrong_register_classes_are_rejected_after_alias_resolution() {
    for declaration in [
        "Texture2D<float> a:register(u0);",
        "RWTexture2D<float> a:register(t0);",
        "StructuredBuffer<float> a:register(b0);",
        "RWStructuredBuffer<float> a:register(s0);",
        "ByteAddressBuffer a:register(u0);",
        "RWByteAddressBuffer a:register(t0);",
        "SamplerState a:register(t0);",
        "SamplerComparisonState a:register(b0);",
        "struct P{float x;};ConstantBuffer<P> a:register(t0);",
        "typedef RWTexture2D<float> Image;Image a[2]:register(t0);",
    ] {
        reject(declaration, "T022");
    }
}

#[test]
fn malformed_or_overflowing_register_annotations_are_rejected() {
    for binding in [
        "T0",
        "t",
        "t-1",
        "t1x",
        "t1,s2",
        "t1,space",
        "t1,SPACE2",
        "t1,space2,extra",
        "t2147483648",
        "t4294967295",
        "t99999999999999999999999",
        "t0,space2147483648",
        "t0,space99999999999999999999999",
        "t0.",
        "t0,space2.",
    ] {
        reject(&format!("Texture2D<float> a:register({binding});"), "S001");
    }
    let compiled = compile("Texture2D<float> a:register(t2147483647,space2147483647);");
    assert_eq!(
        compiled.reflection.globals[0].binding.unwrap().end(),
        2147483648
    );
    reject("Texture2D<float> a[2]:register(t2147483647);", "T022");
    reject("Texture2D<float> a[65536][65536]:register(t0);", "T022");
}

#[test]
fn unsupported_binding_contexts_report_source_errors() {
    for source in [
        "float a:register(c0);",
        "struct P{float x;};P a:register(c0);",
        "void f(){Texture2D<float> a:register(t0);}",
        "void f(){float a:register(c0)=1;}",
        "struct P{Texture2D<float> image;};P a:register(t0);",
    ] {
        reject(source, "T022");
    }
}

#[test]
fn annotations_without_resource_bindings_are_ignored_in_upstream_contexts() {
    let compiled = compile(
        "struct P{float x:register(c2.y);float y:register(b4);};uniform P data:register(b2);float scale:register(t3);static float storage:register(c4)=1;groupshared float shared:register(b7);cbuffer Params{float value:register(c5.y);};float f(float input:register(c2)):register(t2){return input+data.x+scale+value+storage;}",
    );
    assert_eq!(
        compiled
            .reflection
            .global_uniform_binding
            .unwrap()
            .register
            .index,
        0
    );
    assert_eq!(compiled.reflection.global_uniform_size, Some(12));
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|g| g.name == "Params")
            .unwrap()
            .binding
            .unwrap()
            .register
            .index,
        1
    );
    assert!(
        compiled
            .reflection
            .globals
            .iter()
            .filter(|g| g.name != "Params")
            .all(|g| g.binding.is_none())
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("register(c"));
    assert!(!code.contains("register(b2)"));
    assert!(!code.contains("register(t3)"));
    assert!(code.contains("float f(float input)"));
}

#[test]
fn imports_includes_macros_and_target_renaming_preserve_binding_identity() {
    let includes = MemoryIncludes{files: [
        ("resources.slang".into(), "module resources;public typedef Texture2D<float> Image;public Image line:register(t3,space5);".into()),
        ("local.slang".into(), "RWStructuredBuffer<float> register:register(u4,space5);".into()),
    ].into()};
    let compiled = compile_source(
        "main.slang",
        "import resources;\n#define SLOT s2\n#include \"local.slang\"\nSamplerState sampler:register(SLOT,space5);float f(){register[0]=line[uint2(0)];return register[0];}",
        &CompileOptions::default(),
        &includes,
    ).unwrap();
    assert!(compiled.diagnostics.is_empty());
    let code = compiled.artifact.unwrap();
    let imported = compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.module.as_deref() == Some("resources"))
        .unwrap();
    assert_eq!(imported.name, "line");
    assert_eq!(imported.binding.unwrap().register.index, 3);
    assert!(code.contains(&format!("{} : register(t3, space5)", imported.target_name)));
    let root = compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "register")
        .unwrap();
    assert_ne!(root.target_name, "register");
    assert!(code.contains(&format!("{} : register(u4, space5)", root.target_name)));
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|g| g.name == "sampler")
            .unwrap()
            .binding
            .unwrap()
            .register
            .index,
        2
    );
}

#[test]
fn reflection_serializes_binding_origins_and_omits_private_storage_bindings() {
    let compiled = compile(
        "Texture2D<float> a[2]:register(t3,space4),b;static float value=1;float f(){static float cache=value;return cache;}",
    );
    let reflection = serde_json::to_value(&compiled.reflection).unwrap();
    assert_eq!(reflection["schema_version"], 11);
    assert_eq!(
        reflection["globals"][0]["binding"],
        serde_json::json!({"class":"t","index":3,"space":4,"count":2,"explicit":true})
    );
    assert_eq!(
        reflection["globals"][1]["binding"],
        serde_json::json!({"class":"t","index":0,"space":0,"count":1,"explicit":false})
    );
    assert!(reflection["globals"][2].get("binding").is_none());
    assert_eq!(compiled.reflection.globals.len(), 3);
    assert!(compiled.ir.globals.len() > 3);
}

#[test]
fn overlap_warnings_use_presumed_source_locations() {
    let compiled = compile(
        "Texture2D<float> a:register(t0);\n#line 90 \"bindings.generated\"\nTexture2D<float> b:register(t0);",
    );
    assert_eq!(compiled.diagnostics.len(), 1);
    let message = compiled.diagnostics[0].render(&compiled.sources);
    assert!(message.contains("bindings.generated:90:20"), "{message}");
}
