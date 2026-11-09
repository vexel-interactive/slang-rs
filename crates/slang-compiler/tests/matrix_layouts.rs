use slang_compiler::{CompileOptions, MatrixMajor, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "major.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn row_and_column_layouts_share_value_types_but_keep_distinct_uniform_layouts() {
    let compiled = compile(
        "struct S{row_major float2x3 row;column_major float2x3 column;};ConstantBuffer<S> params;row_major float2x3 values[2];float tail;",
    );
    assert_eq!(compiled.reflection.schema_version, 11);
    let row = &compiled.reflection.uniform_types["row_major float2x3"];
    let column = &compiled.reflection.uniform_types["float2x3"];
    assert_eq!(
        (row.size, row.matrix_stride, row.matrix_major),
        (28, Some(16), Some(MatrixMajor::Row))
    );
    assert_eq!(
        (column.size, column.matrix_stride, column.matrix_major),
        (40, Some(16), Some(MatrixMajor::Column))
    );
    let structure = &compiled.reflection.uniform_types["S"];
    assert_eq!(structure.size, 72);
    assert_eq!(
        structure
            .fields
            .iter()
            .map(|f| f.offset)
            .collect::<Vec<_>>(),
        [0, 32]
    );
    let values = &compiled.reflection.uniform_types["row_major float2x3[2]"];
    assert_eq!((values.size, values.array_stride), (60, Some(32)));
    assert_eq!(values.element_type.as_deref(), Some("row_major float2x3"));
    assert_eq!(compiled.reflection.global_uniform_size, Some(64));
}

#[test]
fn layout_qualifiers_preserve_algebra_aliases_and_last_modifier_selection() {
    let compiled = compile(
        "typedef float2x3 M;row_major column_major M data;row_major M copy(row_major M input){row_major M local=input;return local;}float f(){return copy(data)[0][1];}",
    );
    assert_eq!(compiled.reflection.globals[0].type_name, "float2x3");
    assert_eq!(
        compiled.reflection.globals[0]
            .uniform_layout
            .as_ref()
            .unwrap()
            .type_name,
        "float2x3"
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("row_major float2x3 copy(row_major float2x3 input)"));
    assert!(code.contains("column_major float2x3 data;"));
    assert!(!code.contains("row_major column_major"));
}

#[test]
fn qualified_numeric_casts_preserve_the_logical_matrix_shape() {
    let compiled = compile(
        "float2x3 copy(float2x3 value){return (row_major column_major float2x3)value;}float2x3 make(){return (column_major matrix<float,2,3>)1;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((float2x3)value)"));
    assert!(code.contains("((float2x3)1)"));
}

#[test]
fn packed_matrix_values_keep_their_row_major_storage_when_loaded() {
    let compiled = compile(
        "struct S{row_major float2x3 m:packoffset(c2);float tail;};ConstantBuffer<S> params;float f(ConstantBuffer<S> input){return input.m[0][1]+input.tail;}",
    );
    assert_eq!(compiled.reflection.uniform_types["S"].size, 64);
    assert_eq!(
        compiled.reflection.uniform_types["S"].fields[0].type_name,
        "row_major float2x3"
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("row_major float2x3 _slang__slang_uniform_view_0 : packoffset(c2);"));
}

#[test]
fn single_row_matrices_start_on_registers_and_preserve_tail_space() {
    let compiled = compile(
        "float prefix;row_major float1x1 first;row_major float1x2 second;float tail;column_major float2x1 column;",
    );
    let offsets: Vec<_> = compiled
        .reflection
        .globals
        .iter()
        .map(|g| g.uniform_layout.as_ref().unwrap().offset)
        .collect();
    assert_eq!(offsets, [0, 16, 32, 40, 48]);
    assert_eq!(compiled.reflection.global_uniform_size, Some(56));
}

#[test]
fn qualified_aliases_preserve_storage_through_forwards_arrays_and_overrides() {
    let compiled = compile(
        "typealias Rows=Row[2];typedef row_major float2x3 Row;typedef column_major Row Column;struct S{Rows rows;Column column;row_major Column override;};ConstantBuffer<S> params;Row copy(Row input){Row local=input;return local;}",
    );
    let structure = &compiled.reflection.uniform_types["S"];
    assert_eq!(structure.size, 140);
    assert_eq!(structure.fields[0].type_name, "row_major float2x3[2]");
    assert_eq!(structure.fields[1].type_name, "float2x3");
    assert_eq!(structure.fields[2].type_name, "row_major float2x3");
    let code = compiled.artifact.unwrap();
    assert!(code.contains("row_major float2x3 copy(row_major float2x3 input)"));
    assert!(code.contains("column_major float2x3 column;"));
    assert!(code.contains("row_major float2x3 override;"));
}

#[test]
fn imported_aliases_retain_matrix_layout_in_buffers_and_parameters() {
    let compiled = compile_source(
        "main.slang",
        "import library;struct S{Rows rows;};ConstantBuffer<S> params;Row copy(Row value){return value;}",
        &CompileOptions::default(),
        &MemoryIncludes {
            files: std::collections::BTreeMap::from([(
                "library.slang".into(),
                "module library;public typedef row_major float2x3 Row;public typealias Rows=Row[2];".into(),
            )]),
        },
    ).unwrap();
    assert_eq!(compiled.reflection.uniform_types["S"].size, 60);
    assert_eq!(
        compiled.reflection.uniform_types["S"].fields[0].type_name,
        "row_major float2x3[2]"
    );
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("row_major float2x3 copy(row_major float2x3 value)")
    );
}

#[test]
fn default_major_controls_nested_uniform_layouts_and_explicit_overrides() {
    let source = "struct S{float2x3 implicit;column_major float2x3 column;};ConstantBuffer<S> params;float2x3 data;";
    for (major, first, size, data_size) in [
        (MatrixMajor::Row, "row_major float2x3", 72, 28),
        (MatrixMajor::Column, "float2x3", 88, 40),
    ] {
        let compiled = compile_source(
            "default.slang",
            source,
            &CompileOptions {
                matrix_major: major,
                ..CompileOptions::default()
            },
            &MemoryIncludes::default(),
        )
        .unwrap();
        assert_eq!(compiled.reflection.matrix_major, major);
        assert_eq!(compiled.reflection.uniform_types["S"].size, size);
        assert_eq!(
            compiled.reflection.uniform_types["S"].fields[0].type_name,
            first
        );
        assert_eq!(
            compiled.reflection.uniform_types["S"].fields[1].type_name,
            "float2x3"
        );
        assert_eq!(compiled.reflection.global_uniform_size, Some(data_size));
        let pragma = if major == MatrixMajor::Row {
            "row_major"
        } else {
            "column_major"
        };
        assert!(
            compiled
                .artifact
                .unwrap()
                .starts_with(&format!("#pragma pack_matrix({pragma})"))
        );
    }
}

#[test]
fn packed_columns_remain_column_major_under_a_row_major_default() {
    let compiled = compile_source(
        "packed.slang",
        "struct S{column_major float2x3 m:packoffset(c2);float2x3 r;};ConstantBuffer<S> params;",
        &CompileOptions {
            matrix_major: MatrixMajor::Row,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(compiled.reflection.uniform_types["S"].size, 108);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("column_major float2x3 _slang__slang_uniform_view_0 : packoffset(c2);"));
    assert!(code.contains("row_major float2x3 _slang__slang_uniform_view_1 : packoffset(c5);"));
}

#[test]
fn matrix_array_buffer_reads_evaluate_indices_once_and_preserve_writable_locations() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/matrix-layout-resource-arrays.slang"
    ));
    let code = compiled.artifact.unwrap();
    // Prototype, definition, and the two source calls; reconstruction uses the
    // helper's index parameter instead of copying either source call.
    assert_eq!(code.matches("nextIndex()").count(), 4);
    assert!(code.lines().any(|line| line.trim().starts_with("set(")
        && line.contains("_slang_effect_")
        && !line.contains("nextIndex()")));
    assert!(code.contains("InterlockedAdd(((writable)[0]).tag, 1u)"));
    assert!(!code.contains("values[2] = ((input)[nextIndex()]).rows"));
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .filter(|g| g.resource_access.is_some())
            .count(),
        3
    );
}

#[test]
fn resource_element_qualifiers_preserve_aliases_overloads_and_index_evaluation() {
    let source =
        include_str!("../../../tests/conformance/positive/matrix-layout-resource-elements.slang");
    for major in [MatrixMajor::Row, MatrixMajor::Column] {
        let compiled = compile_source(
            "elements.slang",
            source,
            &CompileOptions {
                matrix_major: major,
                ..CompileOptions::default()
            },
            &MemoryIncludes::default(),
        )
        .unwrap();
        let rows = compiled
            .reflection
            .globals
            .iter()
            .find(|g| g.name == "rows")
            .unwrap();
        let expected = if major == MatrixMajor::Row {
            "StructuredBuffer<float2x3>"
        } else {
            "StructuredBuffer<row_major float2x3>"
        };
        assert_eq!(rows.type_name, expected);
        assert_eq!(rows.resource_access.as_deref(), Some("read"));
        let code = compiled.artifact.unwrap();
        assert_eq!(code.matches("nextIndex()").count(), 3);
        assert!(!code.contains("StructuredBuffer<row_major"));
        assert!(!code.contains("StructuredBuffer<column_major"));
        assert_eq!(compiled.ir.structures.len(), 1);
    }
}

#[test]
fn explicitly_matching_resource_layouts_convert_to_the_default_resource_type() {
    for (major, qualifier) in [
        (MatrixMajor::Row, "row_major"),
        (MatrixMajor::Column, "column_major"),
    ] {
        let source = format!(
            "StructuredBuffer<{qualifier} float2x3> input;float read(StructuredBuffer<float2x3> values){{return values[0][1][2];}}float f(){{return read(input);}}"
        );
        compile_source(
            "matching.slang",
            &source,
            &CompileOptions {
                matrix_major: major,
                ..CompileOptions::default()
            },
            &MemoryIncludes::default(),
        )
        .unwrap();
    }
}

#[test]
fn matrix_array_materialization_is_bounded_in_uniform_and_structured_storage() {
    for source in [
        "row_major float2x3 values[100000];",
        "struct S{row_major float2x3 values[100000];};StructuredBuffer<S> input;float f(){return input[0].values[0][0][0];}",
    ] {
        let error = compile_source(
            "huge.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "I004"));
    }
}

#[test]
fn layout_modifiers_require_matrix_shapes_in_each_declaration_context() {
    for source in [
        "row_major float value;",
        "struct S{column_major float2 value;};",
        "void f(row_major int value){}",
        "row_major float f(){return 0;}",
        "void f(){column_major bool value=true;}",
        "struct S{float x;};row_major S data;",
        "typedef row_major float Value;",
        "typealias Value=column_major int[2];",
        "StructuredBuffer<row_major float> data;",
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T001"),
            "{source}"
        );
    }
}
