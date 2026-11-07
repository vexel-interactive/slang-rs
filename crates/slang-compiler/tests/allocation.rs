use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "allocation.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn index(compiled: &Compilation, name: &str) -> u32 {
    compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.name == name)
        .unwrap()
        .binding
        .unwrap()
        .register
        .index
}

#[test]
fn explicit_ranges_are_reserved_before_any_automatic_assignment() {
    let compiled =
        compile("Texture2D<float> a;Texture2D<float> fixed:register(t1);Texture2D<float> b;");
    assert_eq!(index(&compiled, "a"), 0);
    assert_eq!(index(&compiled, "fixed"), 1);
    assert_eq!(index(&compiled, "b"), 2);
    assert!(!compiled.reflection.globals[0].binding.unwrap().explicit);
    assert!(compiled.reflection.globals[1].binding.unwrap().explicit);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("a : register(t0)"));
    assert!(code.contains("b : register(t2)"));
}

#[test]
fn arrays_use_first_fit_and_leave_smaller_holes_available() {
    let compiled =
        compile("Texture2D<float> a[2];Texture2D<float> fixed:register(t1);Texture2D<float> b;");
    assert_eq!(index(&compiled, "a"), 2);
    assert_eq!(compiled.reflection.globals[0].binding.unwrap().count, 2);
    assert_eq!(index(&compiled, "b"), 0);
}

#[test]
fn automatic_resources_share_their_class_and_default_to_space_zero() {
    let compiled = compile(
        "Texture2D<float> a;StructuredBuffer<float> b;Texture2D<float> c;
        Texture2D<float> fixed:register(t0,space3);SamplerState sampler;
        RWByteAddressBuffer bytes;RWStructuredBuffer<float> output;",
    );
    assert_eq!(index(&compiled, "a"), 0);
    assert_eq!(index(&compiled, "b"), 1);
    assert_eq!(index(&compiled, "c"), 2);
    assert_eq!(index(&compiled, "sampler"), 0);
    assert_eq!(index(&compiled, "bytes"), 0);
    assert_eq!(index(&compiled, "output"), 1);
    assert!(
        compiled
            .reflection
            .globals
            .iter()
            .filter(|g| g.name != "fixed")
            .all(|g| g.binding.unwrap().register.space == 0)
    );
}

#[test]
fn implicit_uniform_storage_reserves_a_constant_buffer_before_resources() {
    for (source, expected) in [
        (
            "float value;ConstantBuffer<P> a;ConstantBuffer<P> b;",
            [1, 2],
        ),
        (
            "float value;ConstantBuffer<P> a:register(b0);ConstantBuffer<P> b;",
            [0, 2],
        ),
        (
            "static float value=1;ConstantBuffer<P> a;ConstantBuffer<P> b;",
            [0, 1],
        ),
        (
            "groupshared float value;ConstantBuffer<P> a;ConstantBuffer<P> b;",
            [0, 1],
        ),
    ] {
        let compiled = compile(&format!("struct P{{float x;}};{source}"));
        assert_eq!([index(&compiled, "a"), index(&compiled, "b")], expected);
        assert!(
            compiled
                .reflection
                .globals
                .iter()
                .find(|g| g.name == "value")
                .unwrap()
                .binding
                .is_none()
        );
    }
}

#[test]
fn root_module_resources_precede_imports_and_unused_resources_keep_their_slots() {
    let includes = MemoryIncludes {
        files: [(
            "library.slang".into(),
            "module library;public Texture2D<float> imported;".into(),
        )]
        .into(),
    };
    let compiled = compile_source("root.slang", "import library;Texture2D<float> unused;Texture2D<float> root;float f(){return root[uint2(0)]+imported[uint2(0)];}", &CompileOptions::default(), &includes).unwrap();
    assert_eq!(index(&compiled, "unused"), 0);
    assert_eq!(index(&compiled, "root"), 1);
    assert_eq!(index(&compiled, "imported"), 2);
    let again = compile("Texture2D<float> root;");
    assert_eq!(index(&again, "root"), 0);
}

#[test]
fn overlapping_explicit_ranges_are_reserved_as_a_union() {
    let compiled = compile("Texture2D<float> a[4]:register(t0),b[4]:register(t2),automatic;");
    assert_eq!(index(&compiled, "automatic"), 6);
    assert_eq!(compiled.diagnostics.len(), 1);
    assert_eq!(compiled.diagnostics[0].code, "T023");
}

#[test]
fn exhausted_ranges_fail_without_expanding_large_resource_arrays() {
    for source in [
        "RWByteAddressBuffer all[2147483648]:register(u0);RWByteAddressBuffer extra;",
        "struct P{float x;};ConstantBuffer<P> all[2147483648]:register(b0);float value;",
        "Texture2D<float> all[65536][65536];",
    ] {
        let error = compile_source(
            "exhausted.slang",
            source,
            &CompileOptions {
                check_only: true,
                ..CompileOptions::default()
            },
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "T022"));
    }
}
