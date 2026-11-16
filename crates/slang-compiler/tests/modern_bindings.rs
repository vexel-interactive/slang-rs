use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "modern-bindings.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn modern_bindings_infer_literals_numeric_shapes_arrays_and_member_defaults() {
    compile(include_str!("../../../tests/conformance/positive/modern-binding-values.slang"));
}

#[test]
fn modern_bindings_preserve_generic_parameter_and_owner_identity() {
    compile(include_str!("../../../tests/conformance/positive/modern-binding-generics.slang"));
}

#[test]
fn inferred_global_types_follow_forward_dependencies_and_initializer_calls() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/modern-binding-globals.slang"));
    for name in ["first", "second", "computed", "Count"] {
        let global = compiled.reflection.globals.iter().find(|global| global.name == name).unwrap();
        assert_eq!(global.type_name, "int", "{name}");
    }
}

#[test]
fn global_initializers_follow_direct_and_transitive_function_dependencies() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/modern-binding-global-order.slang"
    ));
    let artifact = compiled.artifact.unwrap();
    let source = artifact.find("static int source =").unwrap();
    let biased = artifact.find("static int biased =").unwrap();
    let computed = artifact.find("static int computed =").unwrap();
    assert!(source < biased && biased < computed, "{artifact}");
}

#[test]
fn global_initializer_order_preserves_unrelated_side_effects_after_dependencies() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/modern-binding-global-effects.slang"
    ));
    let globals: Vec<_> = compiled.ir.globals.iter().map(|global| global.name.as_str()).collect();
    assert_eq!(&globals[..4], ["trace", "second", "first", "unrelated"]);
}

#[test]
fn global_initializer_calls_preserve_later_static_local_guards() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/modern-binding-global-static-helper.slang"
    ));
    let artifact = compiled.artifact.unwrap();
    let first = artifact.find("static int first =").unwrap();
    let guard = artifact.find("initialized_1 = false").unwrap();
    assert!(guard < first, "{artifact}");
}

#[test]
fn explicit_global_initializer_cycles_report_diagnostics() {
    for source in [
        r#"
static int first = read();
static int second = first + 1;
int read() { return second; }
void run() {}
"#,
        r#"
static int value = initialize();
int initialize() {
    value = 2;
    return value + 1;
}
void run() {}
"#,
    ] {
        let failure = compile_source(
            "cycle.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            failure.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "I002"
                    && diagnostic.message.contains("cyclic global initializer")
            }),
            "{failure}"
        );
    }
}

#[test]
fn modern_resource_bindings_keep_handle_types_and_read_write_contents() {
    compile(include_str!("../../../tests/conformance/positive/modern-binding-resources.slang"));
}

#[test]
fn modern_bindings_preserve_local_static_initializers_and_unused_effects() {
    compile(include_str!("../../../tests/conformance/positive/modern-binding-effects.slang"));
}

#[test]
fn mixed_memberwise_initialization_leaves_immutable_fields_and_their_defaults_untouched() {
    compile(include_str!(
        "../../../tests/conformance/positive/modern-binding-partial-initialization.slang"
    ));
}

#[test]
fn cbuffer_member_inference_keeps_uniform_value_shapes() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/modern-binding-cbuffer.slang"));
    assert!(compiled.artifact.unwrap().contains("register(b3"));
}

#[test]
fn inferred_unit_bindings_preserve_initializer_assignment_and_member_effects() {
    compile(include_str!("../../../tests/conformance/positive/modern-binding-unit.slang"));
}

#[test]
fn static_unit_initializers_run_once_without_void_storage() {
    compile(include_str!("../../../tests/native/modern-binding-static-unit.slang"));
}

#[test]
fn generic_unit_arguments_preserve_effects_between_value_arguments() {
    compile(include_str!(
        "../../../tests/conformance/positive/modern-binding-generic-unit-order.slang"
    ));
}

#[test]
fn inferred_member_defaults_read_previously_initialized_receiver_fields() {
    compile(include_str!("../../../tests/native/modern-binding-dependent-default.slang"));
}

#[test]
fn selected_entry_preserves_global_unit_startup_effects() {
    compile_source(
        "startup.slang",
        include_str!("../../../tests/native/modern-binding-global-unit.slang"),
        &CompileOptions {
            entry_point: Some("main".into()),
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
}

#[test]
fn imported_modern_members_and_initializers_keep_declaration_scope() {
    let includes = MemoryIncludes {
        files: [(
            "library.slang".into(),
            include_str!("../../../tests/conformance/positive/modern-binding-module/library.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/modern-binding-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
}

#[test]
fn immutable_bindings_fields_and_output_arguments_reject_writes() {
    for source in [
        include_str!("../../../tests/conformance/negative/modern-binding-let-write.slang"),
        include_str!("../../../tests/conformance/negative/modern-binding-let-field-write.slang"),
        include_str!("../../../tests/conformance/negative/modern-binding-let-output.slang"),
    ] {
        let failure = compile_source(
            "immutable.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            failure.diagnostics.iter().any(|diagnostic| diagnostic.code == "T015"),
            "{failure}"
        );
    }
}

#[test]
fn omitted_types_need_an_initializer_and_let_is_not_a_compile_time_constant() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/modern-binding-missing-type.slang"),
            "T034",
        ),
        (
            include_str!("../../../tests/conformance/negative/modern-binding-let-dimension.slang"),
            "T002",
        ),
    ] {
        let failure = compile_source(
            "invalid-type.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(failure.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{failure}");
    }
}

#[test]
fn inferred_global_type_cycles_report_diagnostics() {
    let failure = compile_source(
        "cycle.slang",
        include_str!("../../../tests/conformance/negative/modern-binding-global-cycle.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(failure.diagnostics.iter().any(|diagnostic| diagnostic.code == "T034"), "{failure}");
}
