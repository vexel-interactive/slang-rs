use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "static.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn shared_static_storage_keeps_source_names_and_instance_layouts() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/struct-static-fields.slang"
    ));
    let names: Vec<_> = result
        .reflection
        .globals
        .iter()
        .map(|g| g.name.as_str())
        .collect();
    for name in [
        "State.value",
        "State.bias",
        "State.computed",
        "State.fromMethod",
        "State.pair",
        "State.data",
        "Effect.value",
        "A_B.c",
        "A.B_c",
    ] {
        assert!(names.contains(&name), "{names:?}");
    }
    let state = result
        .ir
        .structures
        .iter()
        .find(|s| s.name == "State")
        .unwrap();
    assert_eq!(state.fields.len(), 1);
    assert_eq!(state.fields[0].name, "field");
    let main = result
        .ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let body = serde_json::to_string(&main.body).unwrap();
    assert!(
        !body.contains("make") && !body.contains("constructor_Effect"),
        "{body}"
    );
    assert!(
        !body.contains("AddressView") && !body.contains("BlockExpression"),
        "{body}"
    );
    let value = result
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "State.value")
        .unwrap();
    assert!(value.binding.is_none() && value.uniform_layout.is_none());
}

#[test]
fn forward_static_constants_resolve_in_aliases_signatures_and_templates() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/struct-static-shapes.slang"
    ));
    assert_eq!(
        result.reflection.entry_points[0].thread_group_size,
        Some([3, 1, 1])
    );
    let shape = result
        .ir
        .structures
        .iter()
        .find(|s| s.name == "Shape")
        .unwrap();
    assert_eq!(
        shape.fields[0].ty,
        slang_sema::Type::Array(
            Box::new(slang_sema::Type::Scalar(slang_sema::Scalar::Int)),
            4
        )
    );
}

#[test]
fn static_resource_aliases_and_shared_storage_preserve_layout_and_requirements() {
    let result = compile(include_str!(
        "../../../tests/native/static-resource-members.slang"
    ));
    let data = result
        .ir
        .structures
        .iter()
        .find(|s| s.name == "Data")
        .unwrap();
    assert_eq!(data.fields.len(), 1);
    let settings = result
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "settings")
        .unwrap();
    let layout = settings.uniform_layout.as_ref().unwrap();
    assert_eq!(result.reflection.uniform_types[&layout.type_name].size, 4);
    assert!(
        result
            .reflection
            .globals
            .iter()
            .any(|g| g.name == "Access.sharedValue" && g.binding.is_none())
    );
    let result = compile(include_str!(
        "../../../tests/conformance/positive/struct-static-ignored-base.slang"
    ));
    let main = result
        .ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let body = serde_json::to_string(&main.body).unwrap();
    assert!(
        !body.contains("GroupMemoryBarrier") && !body.contains("make"),
        "{body}"
    );
    assert_eq!(
        result.reflection.entry_points[0].target_profile.as_deref(),
        Some("ps_6_0")
    );
}

#[test]
fn imported_static_fields_enforce_access_and_preserve_module_provenance() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/struct-static-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    let result = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/struct-static-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let value = result
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "Shared.visible")
        .unwrap();
    assert_eq!(value.module.as_deref(), Some("lib"));
    assert_eq!(
        result.reflection.entry_points[0].thread_group_size,
        Some([3, 1, 1])
    );
    for source in [
        include_str!("../../../tests/conformance/positive/struct-static-module/private.slang"),
        include_str!(
            "../../../tests/conformance/positive/struct-static-module/private-shape.slang"
        ),
    ] {
        let error = compile_source("main.slang", source, &CompileOptions::default(), &includes)
            .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T018"),
            "{error}"
        );
    }
}

#[test]
fn invalid_static_fields_report_conflicts_constants_and_receiver_errors() {
    for (source, code) in [
        (
            include_str!(
                "../../../tests/conformance/negative/static-field-missing-initializer.slang"
            ),
            "T004",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-write-const.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-duplicate.slang"),
            "T003",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-method-conflict.slang"),
            "T003",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/static-field-instance-initializer.slang"
            ),
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-cycle.slang"),
            "T026",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-mutable-shape.slang"),
            "T002",
        ),
        (
            include_str!("../../../tests/conformance/negative/static-field-public-owner.slang"),
            "T018",
        ),
        ("struct S{static int x=1;static int x=2;};", "T003"),
        ("int read(){return 3;}static const int x=read();", "T031"),
        (
            "struct S{int field;};void f(){S s={1};int x=s::field;}",
            "T013",
        ),
        (
            "struct S{int read(){return 1;}};void f(){S s={};int x=s::read();}",
            "T014",
        ),
        ("struct S{int x;int read(int n=this.x){return n;}};", "T022"),
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
}

#[test]
fn mutable_member_initializers_and_static_this_access_follow_member_rules() {
    compile(include_str!(
        "../../../tests/conformance/positive/compute-signed-semantics.slang"
    ));
    compile(
        "struct S{static int first=3;static const int second=this.first+1;static int read(){return this.second;}static int third=read();int value=this.second;};static const int x=S.first;[shader(\"compute\")][numthreads(1,1,1)]void main(){}",
    );
    let error = compile_source("invalid.slang", "struct S{static groupshared uint value;};[shader(\"fragment\")]float4 main():SV_Target{return float4(S.value);}", &CompileOptions::default(), &MemoryIncludes::default()).unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T023"),
        "{error}"
    );
}
