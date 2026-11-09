use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "generic-constructors.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn inferred_constructors_compete_with_memberwise_and_ordinary_overloads() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-constructor-values.slang"
    ));
}

#[test]
fn owner_types_nested_scopes_globals_and_contextual_lists_are_preserved() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-constructor-owners.slang"
    ));
}

#[test]
fn resource_handles_and_resource_valued_structs_use_user_constructors() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-constructor-resources.slang"
    ));
}

#[test]
fn inline_constructors_preserve_loops_returns_and_shared_state() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-constructor-inline.slang"
    ));
}

#[test]
fn imported_constructors_accept_private_caller_arguments_and_enforce_visibility() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!(
                "../../../tests/conformance/positive/generic-constructor-module/lib.slang"
            )
            .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-constructor-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let error = compile_source("main.slang", include_str!("../../../tests/conformance/positive/generic-constructor-module/private-constructor.slang"), &CompileOptions::default(), &includes).unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T014"),
        "{error}"
    );
}

#[test]
fn invalid_arguments_and_unused_opaque_constructor_bodies_are_checked() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/generic-constructor-arity.slang"),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-constructor-no-input.slang"),
            "T014",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-constructor-output-value.slang"
            ),
            "T015",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-constructor-unused-add.slang"
            ),
            "T012",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-constructor-unused-name.slang"
            ),
            "T011",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-constructor-explicit-member.slang"
            ),
            "T002",
        ),
    ] {
        let error = compile_source(
            "invalid.slang",
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
fn generic_constructor_defaults_do_not_initialize_bare_variables_or_fields() {
    let compiled = compile(
        "struct Marker{int value;__init<T=int>(T ignored=T()){value=7;}};struct Outer{Marker field;__init(){}};int run(){Marker local;return Marker().value;}",
    );
    let artifact = compiled.artifact.unwrap();
    assert!(artifact.contains("Marker local;"), "{artifact}");
}

#[test]
fn aggregate_held_constant_buffers_materialize_through_one_handle() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-constructor-resources.slang"
    ));
    let loads = compiled
        .ir
        .functions
        .iter()
        .filter(|f| f.name.contains("cbuffer_value_load"))
        .collect::<Vec<_>>();
    assert_eq!(loads.len(), 1);
    assert_eq!(loads[0].parameters.len(), 1);
    assert!(matches!(
        loads[0].parameters[0].ty,
        slang_sema::Type::ConstantBuffer { .. }
    ));
}

#[test]
fn recursive_constructor_inference_reuses_the_concrete_frontend_instance() {
    let compiled = compile(
        "struct Marker{int value;__init<T>(T input,int depth){if(depth==0){value=3;return;}return Marker(input,depth-1);}};int run(){return Marker(int2(1,2),2).value;}",
    );
    assert_eq!(compiled.ir.functions.len(), 2);
}
