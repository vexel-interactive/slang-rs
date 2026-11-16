use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "adapter-defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn trailing_concrete_defaults_support_numeric_static_and_receiver_adapters() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-default-inputs.slang"
    ));
}

#[test]
fn supplied_interface_parameter_types_rank_concrete_prefixes() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-default-ranking.slang"
    ));
}

#[test]
fn different_default_tail_lengths_do_not_break_conversion_ties() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-default-ambiguous.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-default-ambiguous-tails.slang"
        ),
    ] {
        assert_rejected(source, "T032");
    }
}

#[test]
fn selected_result_arity_and_default_tail_types_are_checked() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-default-drop-required.slang"
        ),
        include_str!("../../../tests/conformance/negative/interface-adapter-default-missing.slang"),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-default-invalid-result.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-adapter-default-late-prototype.slang"
        ),
    ] {
        assert_rejected(source, "T032");
    }
    assert_rejected(
        include_str!("../../../tests/conformance/negative/interface-adapter-default-output.slang"),
        "T022",
    );
}

fn module_includes() -> MemoryIncludes {
    MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!(
                "../../../tests/conformance/positive/interface-adapter-default-module/lib.slang"
            )
            .into(),
        )]
        .into(),
    }
}

#[test]
fn required_defaults_concrete_defaults_and_prototypes_keep_their_declaration_scopes() {
    compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/positive/interface-adapter-default-module/main.slang"
        ),
        &CompileOptions::default(),
        &module_includes(),
    )
    .unwrap();
}

#[test]
fn selected_default_tails_activate_generic_dependencies() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-default-generic-dependency.slang"
    ));
    assert!(compiled.ir.functions.iter().any(|function| function.name.contains("generic")));
}

#[test]
fn unused_generic_owners_do_not_emit_default_tail_dependencies() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/interface-adapter-default-dormant.slang"
    ));
    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "main");
    assert!(compiled.ir.structures.is_empty());
}

#[test]
fn adapted_default_tails_survive_serialization_and_program_rechecking() {
    let source = include_str!(
        "../../../tests/conformance/positive/interface-adapter-default-module/main.slang"
    );
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("main.slang", source);
    let program = slang_module::load(
        root,
        &mut sources,
        &module_includes(),
        &slang_preprocessor::Options::default(),
    )
    .unwrap();
    let checked = slang_sema::check_program(program).unwrap();
    serde_json::to_string(&checked.module).unwrap();
    let rechecked = slang_sema::check_program(slang_module::Program {
        syntax: checked.module.clone(),
        modules: checked.modules.clone(),
        symbols: checked.symbols.clone(),
        diagnostics: vec![],
    })
    .unwrap();
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}

fn assert_rejected(source: &str, code: &str) {
    let error = compile_source(
        "adapter-defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{error}");
}
