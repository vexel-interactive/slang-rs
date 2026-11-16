use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "modern-functions.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn typed_modern_functions_use_existing_defaults_generics_array_returns_and_shader_semantics() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/modern-function-basics.slang"));
    assert_eq!(compiled.reflection.entry_points[0].name, "main");
    assert_eq!(compiled.reflection.entry_points[0].thread_group_size, Some([1, 1, 1]));
    assert!(compiled.artifact.unwrap().contains("SV_DispatchThreadID"));
}

#[test]
fn modern_struct_methods_use_generic_owners_receiver_modes_and_interface_calls() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/modern-function-methods.slang"));
    assert!(compiled.ir.functions.iter().any(|function| function.name == "main"));
    assert!(!compiled.artifact.unwrap().contains("parameter_"));
}

#[test]
fn direct_where_constraints_and_default_interface_bodies_use_existing_witnesses() {
    for source in [
        include_str!("../../../tests/conformance/positive/modern-function-constraints.slang"),
        include_str!("../../../tests/conformance/positive/modern-function-default-interface.slang"),
    ] {
        let compiled = compile(source);
        assert!(!compiled.artifact.unwrap().contains("parameter_"));
    }
}

#[test]
fn imported_modern_function_defaults_keep_declaration_scope() {
    let includes = MemoryIncludes {
        files: [(
            "library.slang".into(),
            include_str!(
                "../../../tests/conformance/positive/modern-function-module/library.slang"
            )
            .into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/modern-function-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert!(compiled.ir.functions.iter().any(|function| function.name == "main"));
    assert!(!compiled.artifact.unwrap().contains("parameter_"));
}

#[test]
fn modern_generic_and_method_functions_preserve_selected_calls_when_rechecked() {
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add(
        "modern-functions.slang",
        include_str!("../../../tests/conformance/positive/modern-function-methods.slang"),
    );
    let program = slang_module::load(
        root,
        &mut sources,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .unwrap();
    let checked = slang_sema::check_program(program).unwrap();
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

#[test]
fn missing_parameter_types_and_values_returned_from_implicit_void_functions_are_rejected() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/modern-function-missing-type.slang"),
            "T034",
        ),
        (
            include_str!("../../../tests/conformance/negative/modern-function-return-value.slang"),
            "T008",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/modern-function-result-semantic.slang"
            ),
            "S001",
        ),
    ] {
        let error = compile_source(
            "invalid-modern-functions.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{error}");
    }
}
