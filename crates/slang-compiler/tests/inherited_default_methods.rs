use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "inherited-defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn inherited_default_fixtures_emit_concrete_functions() {
    for source in [
        include_str!("../../../tests/conformance/positive/interface-default-dispatch.slang"),
        include_str!("../../../tests/conformance/positive/interface-default-name-collision.slang"),
        include_str!(
            "../../../tests/conformance/positive/interface-default-dispatch-arguments.slang"
        ),
        include_str!("../../../tests/conformance/positive/interface-default-dispatch-static.slang"),
        include_str!("../../../tests/conformance/positive/interface-default-diamonds.slang"),
        include_str!("../../../tests/conformance/positive/interface-default-overloads.slang"),
        include_str!("../../../tests/conformance/positive/interface-default-return-dispatch.slang"),
        include_str!(
            "../../../tests/conformance/positive/interface-default-direction-dispatch.slang"
        ),
        include_str!(
            "../../../tests/conformance/positive/interface-default-generic-dispatch.slang"
        ),
        include_str!("../../../tests/conformance/positive/interface-default-generic-origins.slang"),
        include_str!(
            "../../../tests/conformance/positive/interface-default-unrelated-dispatch.slang"
        ),
        include_str!(
            "../../../tests/conformance/positive/interface-default-concrete-override.slang"
        ),
        include_str!(
            "../../../tests/conformance/positive/interface-default-abstract-override.slang"
        ),
    ] {
        let compiled = compile(source);
        let artifact = compiled.artifact.unwrap();

        assert!(!artifact.contains("parameter_"));
        assert!(!artifact.contains("default_lookup"));
        assert!(compiled.ir.functions.iter().any(|function| function.name == "main"));
    }
}

#[test]
fn unrelated_defaults_keep_explicit_and_inferred_witnesses_separate() {
    let source = include_str!(
        "../../../tests/conformance/positive/interface-default-unrelated-dispatch.slang"
    );
    compile(source);
    compile(
        &source
            .replace("left(item)", "left<Item>(item)")
            .replace("right(item)", "right<Item>(item)"),
    );

    let source =
        include_str!("../../../tests/conformance/positive/interface-default-generic-origins.slang");
    compile(
        &source
            .replace("first(item)", "first<Item>(item)")
            .replace("second(item)", "second<Item>(item)"),
    );
}

#[test]
fn failed_opaque_selection_does_not_publish_an_arbitrary_witness() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-default-unrelated-direct.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-default-unrelated-constraints.slang"
        ),
    ] {
        assert_rejected(source, "T014");
    }
}

#[test]
fn abstract_redeclarations_require_a_concrete_implementation() {
    assert_rejected(
        include_str!(
            "../../../tests/conformance/negative/interface-default-abstract-redeclared.slang"
        ),
        "T032",
    );
    compile(include_str!(
        "../../../tests/conformance/positive/interface-default-abstract-override.slang"
    ));
}

#[test]
fn interface_constructor_bodies_are_rejected() {
    assert_rejected(
        r#"
            interface I {
                __init() {
                }
            }
        "#,
        "T001",
    );
}

#[test]
fn adapted_same_name_overrides_are_diagnosed_instead_of_using_the_default() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-default-adapted-override.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-default-adapted-missing-override.slang"
        ),
    ] {
        assert_rejected(source, "T032");
    }
}

#[test]
fn specialized_interface_origins_survive_serialization_and_rechecking() {
    let source =
        include_str!("../../../tests/conformance/positive/interface-default-generic-origins.slang");
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("origins.slang", source);
    let tokens = slang_preprocessor::preprocess(
        &mut sources,
        root,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .tokens;
    let checked = slang_sema::check(slang_syntax::parse(&tokens).unwrap()).unwrap();
    let origins: std::collections::BTreeSet<_> = checked
        .module
        .generic_instances
        .iter()
        .filter(|instance| instance.parameter_count == 0 && instance.arguments.len() == 2)
        .map(|instance| instance.arguments[1].name.clone())
        .collect();

    assert_eq!(origins.len(), 2);
    serde_json::to_string(&checked.module).unwrap();
    let rechecked = slang_sema::check(checked.module.clone()).unwrap();

    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}

fn module_includes() -> MemoryIncludes {
    MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!(
                "../../../tests/conformance/positive/interface-default-dispatch-module/lib.slang"
            )
            .into(),
        )]
        .into(),
    }
}

#[test]
fn imported_bodies_and_internal_overloads_keep_their_declaration_module() {
    compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/positive/interface-default-dispatch-module/main.slang"
        ),
        &CompileOptions::default(),
        &module_includes(),
    )
    .unwrap();
}

#[test]
fn ordinary_and_opaque_callers_cannot_access_internal_defaults() {
    for source in [
        include_str!(
            "../../../tests/conformance/negative/interface-default-internal-module/direct.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-default-internal-module/generic.slang"
        ),
    ] {
        let error =
            compile_source("main.slang", source, &CompileOptions::default(), &module_includes())
                .unwrap_err();

        assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T014"), "{error}");
    }
}

#[test]
fn unused_inherited_templates_do_not_activate_default_instances() {
    let compiled = compile(
        r#"
            interface Base {
                int read() {
                    return 3;
                }
            }

            interface Derived : Base {
                int read() {
                    return 7;
                }
            }

            int read<T : Derived>(T value) {
                return value.read();
            }

            int run() {
                return 0;
            }
        "#,
    );

    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "run");
}

fn assert_rejected(source: &str, code: &str) {
    let error = compile_source(
        "inherited-defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();

    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{error}");
}
