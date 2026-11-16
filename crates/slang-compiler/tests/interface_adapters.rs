use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source("adapters.slang", source, &CompileOptions::default(), &MemoryIncludes::default())
        .unwrap()
}

#[test]
fn abstract_result_adapters_are_checked_without_generic_or_default_method_triggers() {
    let source =
        include_str!("../../../tests/conformance/positive/interface-adapter-result-only.slang");
    let compiled = compile(source);
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("result-only.slang", source);
    let program = slang_module::load(
        root,
        &mut sources,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .unwrap();
    let checked = slang_sema::check_program(program).unwrap();
    let adapters: Vec<_> = checked
        .module
        .generic_instances
        .iter()
        .filter(|instance| instance.witness_target.is_some())
        .collect();
    assert_eq!(adapters.len(), 2);
    for result in [slang_sema::Scalar::Int, slang_sema::Scalar::Float] {
        assert!(adapters.iter().any(|instance| {
            compiled.ir.functions.iter().any(|function| {
                function.name == instance.type_name
                    && function.return_type == slang_sema::Type::Scalar(result)
            })
        }));
    }
    let rechecked = slang_sema::check(checked.module.clone()).unwrap();
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}

#[test]
fn interface_adapters_compile_numeric_inputs_results_outputs_and_static_receivers() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/interface-method-adapters.slang"
    ));
    assert!(compiled.ir.functions.iter().any(|function| function.name == "main"));
    assert!(!compiled.artifact.unwrap().contains("parameter_"));
}

#[test]
fn output_integer_views_preserve_signed_operations_and_copy_back() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-output-views.slang"
    ));
}

#[test]
fn weaker_implementation_directions_preserve_writes_and_receiver_values() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-direction-lattice.slang"
    ));
}

#[test]
fn constant_buffer_arguments_are_loaded_before_forwarding_to_concrete_value_parameters() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-cbuffer-input.slang"
    ));
}

#[test]
fn aggregate_alias_parameters_preserve_semantic_identity() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-alias-input.slang"
    ));
}

#[test]
fn interface_signature_selects_an_overload_before_concrete_caller_literals() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-overload-ranking.slang"
    ));
}

#[test]
fn output_types_directions_and_receiver_mutability_are_checked() {
    for source in [
        include_str!("../../../tests/conformance/negative/interface-adapter-output-type.slang"),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-output-direction.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-receiver-direction.slang"
        ),
    ] {
        assert_rejected(source, "T032");
    }
}

#[test]
fn incompatible_results_and_ambiguous_adaptations_are_rejected() {
    for source in [
        include_str!("../../../tests/conformance/negative/interface-adapter-return-type.slang"),
        include_str!("../../../tests/conformance/negative/interface-adapter-ambiguous.slang"),
        include_str!("../../../tests/conformance/negative/interface-adapter-static-priority.slang"),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-ranked-result-mismatch.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-ranked-direction-mismatch.slang"
        ),
    ] {
        assert_rejected(source, "T032");
    }
}

#[test]
fn overloaded_default_adapters_require_override_on_the_concrete_target() {
    assert_rejected(
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-overloaded-missing-override.slang"
        ),
        "T032",
    );
}

fn module_includes() -> MemoryIncludes {
    MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/interface-adapter-module/lib.slang")
                .into(),
        )]
        .into(),
    }
}

#[test]
fn private_adapter_targets_and_requirement_defaults_keep_their_module_scope() {
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/interface-adapter-module/main.slang"),
        &CompileOptions::default(),
        &module_includes(),
    )
    .unwrap();
}

#[test]
fn ordinary_callers_cannot_access_unrelated_private_members() {
    let error = compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-private-member/main.slang"
        ),
        &CompileOptions::default(),
        &module_includes(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T014"), "{error}");
}

#[test]
fn ordinary_public_interface_projection_is_a_documented_native_limit() {
    let error = compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-public-projection/main.slang"
        ),
        &CompileOptions::default(),
        &module_includes(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T014"), "{error}");
}

#[test]
fn adapter_origins_survive_checked_module_serialization_and_rechecking() {
    let source =
        include_str!("../../../tests/conformance/positive/interface-method-adapters.slang");
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("adapters.slang", source);
    let program = slang_module::load(
        root,
        &mut sources,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .unwrap();
    let checked = slang_sema::check_program(program).unwrap();
    assert!(
        checked.module.generic_instances.iter().any(|instance| instance.witness_target.is_some())
    );
    serde_json::to_string(&checked.module).unwrap();
    let rechecked = slang_sema::check(checked.module.clone()).unwrap();
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}

fn assert_rejected(source: &str, code: &str) {
    let error = compile_source(
        "adapters.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{error}");
}
