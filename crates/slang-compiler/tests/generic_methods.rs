use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "generic-methods.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn method_arguments_defaults_overloads_and_output_locations_are_specialized() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-method-values.slang"
    ));
    let artifact = compiled.artifact.unwrap();
    assert!(!artifact.contains("function_call_"), "{artifact}");
    assert!(!artifact.contains("parameter_"), "{artifact}");
}

#[test]
fn arbitrary_receivers_and_nested_type_applications_preserve_call_identity() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-method-receivers.slang"
    ));
    compile(
        "struct Box{T copy<T>(T input){return input;}};Box create<T>(T input){Box box={};return box;}T copy<T>(T input){return T();}int run(){return create(3).copy<int>(4);}",
    );
}

#[test]
fn method_and_enclosing_owner_arguments_have_independent_specializations() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-method-owners.slang"
    ));
    let states = compiled
        .reflection
        .globals
        .iter()
        .filter(|g| g.name.ends_with(".count"))
        .collect::<Vec<_>>();
    assert_eq!(states.len(), 2);
    assert_ne!(states[0].target_name, states[1].target_name);
}

#[test]
fn resource_handles_and_aggregate_results_survive_method_calls() {
    compile(include_str!(
        "../../../tests/conformance/positive/generic-method-resources.slang"
    ));
}

#[test]
fn imported_methods_accept_private_caller_types_and_enforce_visibility() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/generic-method-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-method-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let error = compile_source(
        "main.slang",
        include_str!(
            "../../../tests/conformance/positive/generic-method-module/private-method.slang"
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

#[test]
fn invalid_method_calls_and_original_opaque_bodies_are_rejected() {
    for (source, code) in [
        (
            "T copy<T>(T input){return input;}struct Other{T copy<T>(T input){return input;}};struct Box{int copy;int read(){return copy(3);}};",
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-unused-add.slang"),
            "T012",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-unused-name.slang"),
            "T011",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-no-input.slang"),
            "T002",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-type-receiver.slang"),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-const-receiver.slang"),
            "T015",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/generic-method-temporary-receiver.slang"
            ),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-output-value.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-unknown-type.slang"),
            "T002",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-duplicate.slang"),
            "T003",
        ),
        (
            include_str!("../../../tests/conformance/negative/generic-method-shadowed.slang"),
            "T014",
        ),
    ] {
        let error = compile_source(
            "generic-methods.slang",
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
fn nested_implicit_calls_respect_member_and_module_scopes() {
    compile(
        "struct Ops{static T copy<T>(T input){return input;}static int initial=copy(3);};int run(){return Ops.initial;}",
    );
    compile(
        "struct Ops<T>{static U copy<U>(U input){return input;}static T initial=copy(T());T value=copy(T());};int run(){Ops<int> ops=Ops<int>();return Ops<int>.initial+ops.value;}",
    );
    compile(
        "int copy(int value){return value+1;}struct Other{T copy<T>(T value){return value;}};struct Outer<T>{static U relay<U>(U value){return value;}struct Inner{T value;T read(){return relay(value);}};};int run(){Outer<int>.Inner inner={2};return inner.read();}",
    );
    compile(
        "int copy(int value){return value;}struct Other{T copy<T>(T value){return value;}};struct Box{int value;int read(){return copy(value);}};int run(){Box box={2};return box.read();}",
    );
}

#[test]
fn explicit_recursive_generic_methods_reuse_their_frontend_instance() {
    let compiled = compile(
        "struct Box{T recurse<T>(T value,int count){if(count==0)return value;return recurse<T>(value,count-1);}};int run(){Box box={};return box.recurse(3,2);}",
    );
    assert_eq!(compiled.ir.functions.len(), 3);
}
