use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn independent_overload_arguments_defaults_and_alias_prototypes_are_specialized() {
    compile_source(
        "defaults.slang",
        include_str!(
            "../../../tests/conformance/positive/generic-function-first-type-default.slang"
        ),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let compiled = compile_source(
        "overloads.slang",
        include_str!("../../../tests/conformance/positive/generic-function-overloads.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("function_call_"), "{code}");
    assert!(!code.contains("symbolic"), "{code}");
}

#[test]
fn equivalent_signatures_late_defaults_and_concrete_ambiguity_fail() {
    for (source, code) in [
        (
            include_str!(
                "../../../tests/conformance/negative/generic-function-ambiguous-specialization.slang"
            ),
            "T014",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-function-duplicate-signature.slang"
            ),
            "T003",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-function-later-type-default.slang"
            ),
            "T002",
        ),
    ] {
        let error = compile_source(
            "overloads.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    }
}

#[test]
fn imported_overload_visibility_and_caller_argument_types_remain_independent() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/generic-overload-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-overload-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let error = compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/positive/generic-overload-module/private-overload.slang"
        ),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T014"),
        "{error}"
    );
}
