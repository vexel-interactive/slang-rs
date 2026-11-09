use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "nested.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn module_files() -> MemoryIncludes {
    MemoryIncludes {
        files: [
            (
                "lib.slang".into(),
                include_str!("../../../tests/conformance/positive/nested-type-module/lib.slang")
                    .into(),
            ),
            (
                "members.slang".into(),
                include_str!(
                    "../../../tests/conformance/positive/nested-type-module/members.slang"
                )
                .into(),
            ),
        ]
        .into(),
    }
}

#[test]
fn nested_constructors_aliases_and_methods_preserve_distinct_types_and_source_names() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/nested-types.slang"
    ));
    let outer = result
        .ir
        .structures
        .iter()
        .find(|s| s.name == "Outer")
        .unwrap();
    assert_eq!(outer.fields.len(), 1);
    let Type::Struct(inner_name) = &outer.fields[0].ty else {
        panic!("{:?}", outer.fields)
    };
    assert_ne!(inner_name, "Inner");
    let inner = result
        .ir
        .structures
        .iter()
        .find(|s| &s.name == inner_name)
        .unwrap();
    assert_eq!(inner.fields.len(), 1);
    assert_eq!(inner.fields[0].ty, Type::Scalar(Scalar::Int));
    assert!(
        result
            .reflection
            .globals
            .iter()
            .any(|g| g.name == "Outer.Inner.shared")
    );
    let main = result
        .ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let body = serde_json::to_string(&main.body).unwrap();
    assert!(
        !body.contains("ignored")
            && !body.contains("AddressView")
            && !body.contains("BlockExpression"),
        "{body}"
    );
}

#[test]
fn qualified_aliases_preserve_shapes_and_matrix_storage_order() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/nested-type-shapes.slang"
    ));
    assert_eq!(
        result.reflection.entry_points[0].thread_group_size,
        Some([3, 1, 1])
    );
    let settings = result
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "settings")
        .unwrap();
    let layout =
        &result.reflection.uniform_types[&settings.uniform_layout.as_ref().unwrap().type_name];
    assert_eq!(layout.fields.len(), 3);
    let matrix = &result.reflection.uniform_types[&layout.fields[2].type_name];
    assert_eq!(matrix.matrix_major, Some(slang_layout::MatrixMajor::Row));
    assert_eq!(matrix.size, 28);
    let transform = result
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "transform")
        .unwrap();
    let matrix =
        &result.reflection.uniform_types[&transform.uniform_layout.as_ref().unwrap().type_name];
    assert_eq!(matrix.matrix_major, Some(slang_layout::MatrixMajor::Row));
    assert_eq!(matrix.size, 28);
}

#[test]
fn nested_type_lookup_excludes_base_effects_and_parameter_dependencies() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/nested-type-ignored-base.slang"
    ));
    assert_eq!(
        result.reflection.entry_points[0].target_profile.as_deref(),
        Some("ps_6_0")
    );
    let main = result
        .ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let body = serde_json::to_string(&main.body).unwrap();
    assert!(
        !body.contains("make") && !body.contains("GroupMemoryBarrier"),
        "{body}"
    );
    compile(
        "struct O{struct I{int value;};};[ForceInline]O make(){return {}; }static O.I value=make().I(2);",
    );
}

#[test]
fn imports_and_module_fragments_keep_nested_metadata_and_private_access() {
    let files = module_files();
    let result = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/nested-type-module/main.slang"),
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    for name in ["Outer.Inner.shared", "Member.Inner.count"] {
        let global = result
            .reflection
            .globals
            .iter()
            .find(|g| g.name == name)
            .unwrap();
        assert_eq!(global.module.as_deref(), Some("lib"));
    }
    assert_eq!(result.reflection.modules[1].files.len(), 2);
    for (source, code) in [
        (
            include_str!(
                "../../../tests/conformance/positive/nested-type-module/private-type.slang"
            ),
            "T018",
        ),
        (
            include_str!(
                "../../../tests/conformance/positive/nested-type-module/private-alias.slang"
            ),
            "T018",
        ),
        (
            include_str!(
                "../../../tests/conformance/positive/nested-type-module/private-method.slang"
            ),
            "T014",
        ),
    ] {
        let error =
            compile_source("main.slang", source, &CompileOptions::default(), &files).unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    }
}

#[test]
fn conflicts_cycles_and_explicit_outer_member_access_fail_before_lowering() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/nested-type-duplicate.slang"),
            "T003",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-field-conflict.slang"),
            "T003",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-method-conflict.slang"),
            "T003",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-alias-cycle.slang"),
            "T002",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-unknown.slang"),
            "T002",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-outer-member.slang"),
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-outer-instance.slang"),
            "T013",
        ),
        (
            "int value=3;struct O{int value;struct I{int read(){return value;}};};",
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/nested-type-shadowed-alias.slang"),
            "T002",
        ),
        (
            "int f(){return 100;}struct O{int f;struct I{int g(){return f();}};};",
            "T014",
        ),
        (
            "struct O{typedef int N;struct I{int N;int f(){return N(3);}};};",
            "T014",
        ),
        (
            "public struct O{internal struct I{};public I field;};",
            "T018",
        ),
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    }
    compile(include_str!(
        "../../../tests/conformance/positive/nested-type-internal-owner.slang"
    ));
}

#[test]
fn nested_declaration_depth_is_bounded() {
    let source = |depth: usize| {
        let mut source = (0..depth)
            .map(|i| format!("struct S{i}{{"))
            .collect::<String>();
        source.push_str(&"};".repeat(depth));
        source
    };
    compile(&source(32));
    let error = compile_source(
        "deep.slang",
        &source(33),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "S001" && d.message.contains("depth")),
        "{error}"
    );
}

#[test]
fn imported_nested_types_require_their_owner_name() {
    let files = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "public struct O{public struct I{};};".into(),
        )]
        .into(),
    };
    for name in ["I", "_slang_nested_type_1_O_I"] {
        let error = compile_source(
            "main.slang",
            &format!("import lib;typealias Leak={name};"),
            &CompileOptions::default(),
            &files,
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T002"),
            "{error}"
        );
    }
}
