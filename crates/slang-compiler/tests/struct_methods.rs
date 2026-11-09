use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "methods.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn method_calls_preserve_selected_receiver_modes_and_normalized_control_flow() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/struct-methods.slang"
    ));
    let json = serde_json::to_string(&result.ir).unwrap();
    assert!(!json.contains("BlockExpression") && !json.contains("AddressView"));
    assert!(!json.contains("MethodCall"), "{json}");
    assert!(result.ir.functions.iter().any(|f| {
        f.parameters
            .first()
            .is_some_and(|p| p.name == "this" && p.modifiers.iter().any(|m| m == "inout"))
    }));
    assert!(result.ir.functions.iter().any(|f| {
        f.parameters
            .first()
            .is_some_and(|p| p.name == "this" && p.modifiers.iter().any(|m| m == "const"))
    }));
    assert!(
        result
            .ir
            .functions
            .iter()
            .all(|f| !matches!(f.return_type, slang_sema::Type::Array(..)))
    );
    for index in 0..result.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&result.ir, index) {
            graph.verify().unwrap();
        }
    }
}

#[test]
fn readonly_and_writable_resource_receivers_lower_with_one_argument_path() {
    let result = compile(include_str!(
        "../../../tests/conformance/positive/struct-methods-resources.slang"
    ));
    let json = serde_json::to_string(&result.ir).unwrap();
    assert!(!json.contains("BlockExpression") && !json.contains("AddressView"));
    assert!(!json.contains("MethodCall"), "{json}");
    assert!(
        result
            .reflection
            .globals
            .iter()
            .any(|g| g.name == "settings" && g.uniform_layout.is_some())
    );
}

#[test]
fn imported_methods_resolve_private_fields_and_declaration_defaults() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/struct-methods-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/struct-methods-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let error = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/struct-methods-module/private.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T014"));
}

#[test]
fn invalid_receivers_and_static_const_calls_report_source_diagnostics() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/method-nonmutating-write.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/method-mutating-const.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/method-mutating-temporary.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/method-static-instance.slang"),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/method-static-field.slang"),
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/method-global-const.slang"),
            "T031",
        ),
        ("struct S{int x;[mutating][nonmutating]void f(){}};", "T006"),
        ("struct S{int x;int f(int y=x){return y;}};", "T022"),
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
fn selected_methods_and_defaults_propagate_entry_point_capabilities() {
    let sources = [
        "struct S{int x;void sync(){GroupMemoryBarrier();}};[shader(\"fragment\")]float4 main():SV_Target{S x={1};x.sync();return float4(1);}",
        "struct S{static void sync(){GroupMemoryBarrier();}};[shader(\"fragment\")]float4 main():SV_Target{S.sync();return float4(1);}",
        "struct S{void sync(){GroupMemoryBarrier();}void relay(){sync();}};[shader(\"fragment\")]float4 main():SV_Target{S().relay();return float4(1);}",
    ];
    for source in sources {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T023"),
            "{error}"
        );
    }
    compile(
        "struct S{int x;int choose(int x){return x;}int choose(float x){GroupMemoryBarrier();return int(x);}};[shader(\"fragment\")]float4 main():SV_Target{return float4(S(1).choose(2));}",
    );
    compile(include_str!(
        "../../../tests/conformance/positive/struct-methods-fragment.slang"
    ));
}
