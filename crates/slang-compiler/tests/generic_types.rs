use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> Compilation {
    compile_source(
        "generic.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn constructors_methods_defaults_and_static_fields_keep_instance_identity() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-types.slang"
    ));
    assert_eq!(compiled.ir.structures.len(), 6);
    let boxes: Vec<_> = compiled
        .ir
        .structures
        .iter()
        .filter(|s| s.fields.iter().any(|f| f.name == "value"))
        .collect();
    assert_eq!(boxes.len(), 2);
    assert!(
        boxes
            .iter()
            .any(|s| s.fields[0].ty == Type::Scalar(Scalar::Int))
    );
    assert!(
        boxes
            .iter()
            .any(|s| s.fields[0].ty == Type::Scalar(Scalar::Float))
    );
    let names: Vec<_> = compiled
        .reflection
        .globals
        .iter()
        .map(|g| g.name.as_str())
        .collect();
    assert!(names.contains(&"Counter<int>.count"), "{names:?}");
    assert!(names.contains(&"Counter<float>.count"), "{names:?}");
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("Box<"));
    assert!(code.contains("inout"));
}

#[test]
fn nested_generics_capture_enclosing_arguments_and_preserve_distinct_owners() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-nested-types.slang"
    ));
    let items: Vec<_> = compiled
        .ir
        .structures
        .iter()
        .filter(|s| s.fields.len() == 2)
        .collect();
    assert_eq!(items.len(), 3);
    for types in [
        [Scalar::Int, Scalar::Float],
        [Scalar::Float, Scalar::Int],
        [Scalar::Int, Scalar::Int],
    ] {
        assert!(
            items
                .iter()
                .any(|s| s.fields[0].ty == Type::Scalar(types[0])
                    && s.fields[1].ty == Type::Scalar(types[1]))
        );
    }
}

#[test]
fn aliases_and_computed_shapes_share_canonical_specializations() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-type-identity.slang"
    ));
    assert_eq!(
        compiled
            .ir
            .structures
            .iter()
            .filter(|s| s.fields.iter().any(|f| f.name == "value"))
            .count(),
        2
    );
}

#[test]
fn resource_and_aggregate_arguments_preserve_handle_types_and_layouts() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-resource-types.slang"
    ));
    let settings = compiled
        .reflection
        .globals
        .iter()
        .find(|g| g.name == "settings")
        .unwrap();
    let layout =
        &compiled.reflection.uniform_types[&settings.uniform_layout.as_ref().unwrap().type_name];
    assert_eq!(layout.size, 4);
    assert!(
        compiled
            .ir
            .structures
            .iter()
            .any(|s| s.fields.iter().any(|f| matches!(f.ty, Type::Buffer { .. })))
    );
}

#[test]
fn imported_templates_accept_caller_private_types_and_reject_private_members() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/generic-type-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-type-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert!(compiled.reflection.modules.iter().any(|m| m.name == "lib"));
    let error = compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/generic-type-module/private-type.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T018"),
        "{error}"
    );
}

#[test]
fn instance_diagnostics_keep_template_source_coordinates() {
    let source = "struct Box<T> {\n T read() { return missing; }\n};\nBox<int> input;";
    let error = compile_source(
        "generic.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "T011").unwrap();
    assert_eq!(diagnostic.span.start, source.find("missing").unwrap());
    assert!(diagnostic.span.context > 0);
    assert!(error.to_string().contains("generic.slang:2:"), "{error}");
}

#[test]
fn invalid_arguments_declarations_and_unbounded_expansion_fail() {
    for (source, code) in [
        ("struct Box<T>{T value;};Box<int,float> input;", "T002"),
        ("struct Box<T>{T value;};Box<> input;", "T002"),
        ("struct Box<T>{};Box<Unknown> input;", "T002"),
        ("struct Box<T>{};Box<3> input;", "T002"),
        ("struct Box<T,T>{};", "T003"),
        ("struct Box<T=Unknown>{};", "T002"),
        ("struct Box<T>{Unknown value;};", "T002"),
        ("struct Box<T>{Box<Box<T>> value;};Box<int> input;", "T002"),
        ("struct Box<T>{Box<T> value;};Box<int> input;", "T002"),
    ] {
        let error = compile_source(
            "generic.slang",
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
fn type_parameters_and_value_bindings_obey_lexical_shadowing() {
    compile(
        "typedef float T; struct Box<T>{T value;T read(int input){return value;}};int run(){Box<int> value={3};return value.read(2);}",
    );
    compile(
        "struct Box<T>{T value;T read(int T){return value;}};int run(){Box<int> value={3};return value.read(2);}",
    );
    compile(
        "struct Box<T>{T value;};int run(){const int width=2;Box<int[width]> first={{1,2}};{const int width=3;Box<int[width]> second={{3,4,5}};}Box<int[2]> last=first;return last.value[1];}",
    );
    compile("bool run(bool a, bool b, bool c){return a < b > (c);}");
}
