use slang_compiler::{CompileOptions, MatrixMajor, MemoryIncludes, compile_source};

#[test]
fn inferred_matrix_aliases_members_and_returns_keep_storage_and_reflection_qualifiers() {
    let source = format!(
        "{}\nConstantBuffer<Item> constants;",
        include_str!("../../../tests/conformance/positive/modern-binding-matrix-major.slang")
    );
    let compiled = compile_source(
        "matrix-major-inference.slang",
        &source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let item = compiled.ir.structures.iter().find(|structure| structure.name == "Item").unwrap();
    assert_eq!(item.fields.len(), 2);
    for field in &item.fields {
        assert!(field.modifiers.iter().any(|modifier| modifier == "row_major"));
    }
    let layout = &compiled.reflection.uniform_types["Item"];
    assert_eq!(layout.fields.len(), 2);
    for field in &layout.fields {
        let matrix = &compiled.reflection.uniform_types[&field.type_name];
        assert_eq!(matrix.matrix_major, Some(MatrixMajor::Row));
        assert_eq!((matrix.size, matrix.matrix_stride), (28, Some(16)));
    }
    let artifact = compiled.artifact.unwrap();
    for name in ["copied", "returned", "copiedReturn"] {
        assert!(artifact.contains(&format!("row_major float2x3 {name}")), "{artifact}");
    }
}
