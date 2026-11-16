use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "constructor-adapters.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn constructor_adapters_preserve_explicit_contextual_automatic_and_default_plans() {
    let source = include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-inputs.slang"
    );
    let compiled = compile(source);
    assert!(compiled.ir.functions.iter().any(|function| function.name == "main"));
    assert!(!compiled.artifact.unwrap().contains("parameter_"));
}

#[test]
fn constructor_output_views_and_weaker_directions_preserve_locations() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-outputs.slang"
    ));
}

#[test]
fn constructor_direction_adapters_keep_input_copies_and_output_locations() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-directions.slang"
    ));
}

#[test]
fn constructor_adapters_support_inherited_generic_interfaces_and_generic_owners() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-owners.slang"
    ));
}

#[test]
fn imported_constructor_adapters_use_private_implementation_defaults_in_declaration_scope() {
    let mut includes = MemoryIncludes::default();
    includes.files.insert(
        "library.slang".into(),
        include_str!(
            "../../../tests/conformance/positive/interface-constructor-adapter-module/library.slang"
        )
        .into(),
    );
    compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/positive/interface-constructor-adapter-module/main.slang"
        ),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
}

#[test]
fn invalid_constructor_witnesses_report_conformance_diagnostics() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-constructor-adapter-missing-tail.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-constructor-adapter-output-type.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-constructor-adapter-direction.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-constructor-adapter-dropped-input.slang"
        ),
    ] {
        let failure = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(failure.diagnostics.iter().any(|diagnostic| diagnostic.code == "T032"));
    }
}

#[test]
fn checked_constructor_adapters_retain_selected_witnesses_when_rechecked() {
    let source = include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-inputs.slang"
    );
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
    let adapters: Vec<_> = checked
        .module
        .generic_instances
        .iter()
        .filter(|instance| instance.witness_target.is_some())
        .collect();
    assert_eq!(adapters.len(), 1);
    let adapter = checked.module.declarations.iter().find_map(|declaration| {
        if let slang_syntax::Declaration::Function(function) = declaration
            && function.name == adapters[0].type_name
        {
            Some(function)
        } else {
            None
        }
    });
    assert!(adapter.unwrap().constructor.is_none());
    let rechecked = slang_sema::check(checked.module.clone()).unwrap();
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}
