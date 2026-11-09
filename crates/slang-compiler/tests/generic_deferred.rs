use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "deferred.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn unselected_bodies_defaults_and_owner_methods_are_not_instantiated() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-deferred-bodies.slang"
    ));
    let artifact = compiled.artifact.unwrap();
    assert!(!artifact.contains("invalid"), "{artifact}");
    assert!(!artifact.contains("Payload"), "{artifact}");
    assert_eq!(compiled.ir.functions.len(), 7);
}

#[test]
fn speculative_signature_dependencies_do_not_leak_into_output_or_reflection() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-deferred-signatures.slang"
    ));
    let artifact = compiled.artifact.unwrap();
    assert!(!artifact.contains("unused"), "{artifact}");
    assert!(!artifact.contains("Payload"), "{artifact}");
    assert_eq!(compiled.ir.functions.len(), 2);
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .map(|g| g.name.as_str())
            .collect::<Vec<_>>(),
        ["input", "output"]
    );
    assert!(compiled.reflection.uniform_types.is_empty());
}

#[test]
fn constructor_initialization_plans_activate_bodies_and_transitive_state() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-deferred-constructors.slang"
    ));
    let state = compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.name.ends_with(".count"))
        .expect("constructor state is reflected");
    let artifact = compiled.artifact.unwrap();
    assert!(artifact.contains(&state.target_name), "{artifact}");
}

#[test]
fn selected_prototype_defaults_activate_nested_generic_calls() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-deferred-defaults.slang"
    ));
}

#[test]
fn implicit_constructor_field_plans_activate_nested_default_constructors() {
    compile(
        "struct Leaf<T>{T value;__init(){value=T();}};struct Root<T>{Leaf<T> leaf;__init(){}};int run(){Root<int> root;return root.leaf.value;}",
    );
}

#[test]
fn imported_private_caller_types_survive_transitive_activation() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/generic-deferred-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-deferred-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
}

#[test]
fn active_ambiguities_and_source_template_errors_are_preserved() {
    for (source, code) in [
        (
            include_str!(
                "../../../tests/conformance/negative/generic-deferred-constructor-ambiguous.slang"
            ),
            "T014",
        ),
        (
            "T pick<T>(T value){return value;}vector<T,2> pick<T>(vector<T,2> value){return value;}int run(){return pick(int2(2,3)).x;}",
            "T014",
        ),
        (
            "T unused<T>(T value){return missing;}int unused(int value){return value;}int run(){return unused(2);}",
            "T011",
        ),
        (
            "T unused<T>(T value){return value+value;}int unused(int value){return value;}int run(){return unused(2);}",
            "T012",
        ),
        (
            "T selected<T>(T value){return T();}StructuredBuffer<int> input;int run(){return selected(input)[0];}",
            "T020",
        ),
    ] {
        let error = compile_source(
            "deferred.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == code),
            "{source}: {error}"
        );
    }
}

#[test]
fn explicit_recursive_specializations_reach_a_frontend_fixed_point() {
    let compiled = compile(
        "T recurse<T>(T value,int count){if(count==0)return value;return recurse<T>(value,count-1);}int run(){return recurse(3,2);}",
    );
    assert_eq!(compiled.ir.functions.len(), 2);
}
