use slang_compiler::{
    Compilation, CompileOptions, MemoryIncludes, UniformTypeLayout, compile_source,
};

fn compile(source: &str) -> Compilation {
    compile_source(
        "layout.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn layout<'a>(compiled: &'a Compilation, name: &str) -> &'a UniformTypeLayout {
    &compiled.reflection.uniform_types[name]
}

#[test]
fn nested_struct_and_array_tails_follow_legacy_constant_buffer_layout() {
    let compiled = compile(
        "struct Pair{float x;float y;};struct P{float z;Pair pair;int b;float2 a[2];int d;};ConstantBuffer<P> p;",
    );
    let p = layout(&compiled, "P");
    assert_eq!((p.size, p.alignment), (60, 16));
    assert_eq!(
        p.fields
            .iter()
            .map(|f| (f.name.as_str(), f.offset))
            .collect::<Vec<_>>(),
        [("z", 0), ("pair", 16), ("b", 24), ("a", 32), ("d", 56)]
    );
    assert_eq!(layout(&compiled, "Pair").size, 8);
    let a = layout(&compiled, "float2[2]");
    assert_eq!((a.size, a.array_stride), (24, Some(16)));
    assert_eq!(a.element_type.as_deref(), Some("float2"));
    let global = &compiled.reflection.globals[0];
    assert_eq!(global.uniform_layout.as_ref().unwrap().type_name, "P");
    assert!(compiled.reflection.global_uniform_size.is_none());
}

#[test]
fn global_uniform_offsets_order_root_values_before_imports_and_exclude_storage() {
    let includes = MemoryIncludes {
        files: [(
            "library.slang".into(),
            "module library;public uniform float imported;".into(),
        )]
        .into(),
    };
    let compiled = compile_source("root.slang", "import library;uniform float3 first;uniform float tail;static float privateValue=1;groupshared float sharedValue;", &CompileOptions::default(), &includes).unwrap();
    assert_eq!(compiled.reflection.global_uniform_size, Some(20));
    let offsets: Vec<_> = compiled
        .reflection
        .globals
        .iter()
        .filter_map(|g| {
            g.uniform_layout
                .as_ref()
                .map(|l| (g.name.as_str(), l.offset))
        })
        .collect();
    assert!(offsets.contains(&("first", 0)));
    assert!(offsets.contains(&("tail", 12)));
    assert!(offsets.contains(&("imported", 16)));
    assert_eq!(offsets.len(), 3);
    let code = compiled.artifact.unwrap();
    assert!(code.find("float3 first;").unwrap() < code.find("float tail;").unwrap());
    let imported = compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "imported")
        .unwrap();
    assert!(
        code.find("float tail;").unwrap()
            < code
                .find(&format!("float {};", imported.target_name))
                .unwrap()
    );
}

#[test]
fn native_numeric_storage_and_single_column_matrices_match_hlsl() {
    let compiled = compile(
        "struct P{float lead;half a;half2 b;bool c;double3 wide;float tail;float3x1 column;float2x3 matrix;float last;};ConstantBuffer<P> p;",
    );
    let p = layout(&compiled, "P");
    assert_eq!(
        p.fields
            .iter()
            .map(|f| (f.name.as_str(), f.offset))
            .collect::<Vec<_>>(),
        [
            ("lead", 0),
            ("a", 4),
            ("b", 8),
            ("c", 16),
            ("wide", 32),
            ("tail", 56),
            ("column", 64),
            ("matrix", 80),
            ("last", 120)
        ]
    );
    assert_eq!(p.size, 124);
    assert_eq!(
        (
            layout(&compiled, "half").size,
            layout(&compiled, "bool").size
        ),
        (4, 4)
    );
    assert_eq!(layout(&compiled, "float3x1").alignment, 4);
    assert_eq!(layout(&compiled, "float2x3").matrix_stride, Some(16));
    assert!(
        compiled
            .artifact
            .unwrap()
            .starts_with("#pragma pack_matrix(column_major)")
    );
}

#[test]
fn array_aliases_resource_arrays_and_check_only_preserve_element_layouts() {
    let compiled = compile_source("arrays.slang", "struct P{float x;float2 a[2][3];float tail;};typealias Data=P;ConstantBuffer<Data> p[2];uniform Data values[2];", &CompileOptions { check_only: true, ..CompileOptions::default() }, &MemoryIncludes::default()).unwrap();
    assert!(compiled.artifact.is_none());
    let p = layout(&compiled, "P");
    assert_eq!(
        p.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 16, 104]
    );
    assert_eq!(p.size, 108);
    assert_eq!(layout(&compiled, "float2[3]").array_stride, Some(16));
    assert_eq!(layout(&compiled, "float2[2][3]").array_stride, Some(48));
    assert_eq!(
        compiled.reflection.globals[0]
            .uniform_layout
            .as_ref()
            .unwrap()
            .type_name,
        "P"
    );
    assert_eq!(compiled.reflection.global_uniform_size, Some(220));
    assert_eq!(layout(&compiled, "P[2]").array_stride, Some(112));
}

#[test]
fn enormous_arrays_and_shared_graphs_are_bounded_without_element_expansion() {
    let compiled = compile_source(
        "large.slang",
        "uniform float values[4294967295];",
        &CompileOptions {
            check_only: true,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.global_uniform_size,
        Some(68_719_476_708)
    );
    assert_eq!(compiled.reflection.uniform_types.len(), 2);
    let error = compile_source(
        "overflow.slang",
        "uniform float values[4294967295][4294967295];",
        &CompileOptions {
            check_only: true,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "I002" && d.message.contains("64-bit"))
    );
    let mut source = String::from("struct S0{float value;};");
    for i in 1..16 {
        source.push_str(&format!("struct S{i}{{S{0} a;S{0} b[2];}};", i - 1));
    }
    source.push_str("uniform S15 data;");
    let compiled = compile(&source);
    assert_eq!(compiled.reflection.uniform_types.len(), 32);
}

#[test]
fn unused_type_graphs_and_private_data_do_not_require_uniform_layout() {
    let compiled = compile(
        "struct P{float x;};static P privateValue;groupshared P sharedValue;RWStructuredBuffer<P> output;",
    );
    assert!(compiled.reflection.uniform_types.is_empty());
    assert!(
        compiled
            .reflection
            .globals
            .iter()
            .all(|g| g.uniform_layout.is_none())
    );
    let json = serde_json::to_value(&compiled.reflection).unwrap();
    assert!(json.get("uniform_types").is_none());
    assert!(json.get("global_uniform_size").is_none());
}
