use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "initialization.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn memberwise_construction_uses_declaration_order_and_single_evaluations() {
    let compiled = compile(
        "struct Pair{float z;int a;};float next(){return 2.0;}float run(){Pair p=Pair(next(),3);return p.z+float(p.a);}",
    );
    let helper = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.return_type.to_string() == "Pair")
        .unwrap();
    let StmtKind::Block(body) = &helper.body.as_ref().unwrap().kind else {
        panic!()
    };
    let fields: Vec<_> = body
        .iter()
        .filter_map(|s| {
            if let StmtKind::Expression(e) = &s.kind
                && let ExprKind::Binary { left, .. } = &e.kind
                && let ExprKind::Member { name, .. } = &left.kind
            {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(fields, ["z", "a"]);
    assert_eq!(compiled.artifact.unwrap().matches("next()").count(), 3);
}

#[test]
fn omitted_fields_use_defaults_and_nested_array_element_defaults() {
    let compiled=compile("struct Pair{float z=7;int a=3;};struct Container{Pair items[2];float value=4;};
        float run(){Pair p=Pair(2);Container c=Container();return p.z+float(p.a)+c.items[1].z+c.value;}");
    assert!(
        compiled
            .ir
            .structures
            .iter()
            .all(|s| s.fields.iter().all(|f| f.initializer.is_none()))
    );
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.return_type.to_string() == "Container")
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("[1u]"));
    assert!(code.contains("((float)7)"));
}

#[test]
fn copy_construction_does_not_call_field_defaults() {
    let compiled = compile(
        "float next(){return 3;}struct Pair{float value=next();};float run(){Pair p=Pair();Pair q=Pair(p);return q.value;}",
    );
    assert_eq!(compiled.artifact.unwrap().matches("next()").count(), 3);
    assert_eq!(
        compiled
            .ir
            .functions
            .iter()
            .filter(|f| f.return_type.to_string() == "Pair")
            .count(),
        1
    );
}

#[test]
fn public_memberwise_parameters_exclude_internal_fields_even_within_owner() {
    let compiled = compile(
        "module test;public struct Pair{public float x;float hidden=3;};float run(){Pair p=Pair(1);return p.hidden;}",
    );
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.return_type.to_string() == "Pair")
    );
    let source = "module test;public struct Pair{public float x;float hidden=3;};float run(){Pair p=Pair(1,2);return p.x;}";
    let error = compile_source(
        "bad.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T014"));
    let files = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public struct Pair{public float x;float hidden=3;};".into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        "import lib;float run(){Pair p=Pair(1);Pair q=Pair(p);return q.x;}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
}

#[test]
fn generated_constructor_names_cannot_capture_local_bindings() {
    let compiled = compile(
        "struct Pair{float value;};float run(){float _slang_init_0=2;Pair p=Pair(1);return p.value+_slang_init_0;}",
    );
    let helper = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.return_type.to_string() == "Pair")
        .unwrap();
    assert_ne!(helper.name, "_slang_init_0");
    assert!(compiled.artifact.unwrap().contains("_slang__slang_init__"));
}

#[test]
fn invalid_defaults_and_recursive_expansion_report_errors() {
    for (source, code) in [
        ("struct Pair{float2 value=float3(1);};", "T008"),
        ("struct Pair{float value=missing;};", "T011"),
        (
            "struct Pair{float value;};void run(){Pair p=Pair(1,2);}",
            "T014",
        ),
        (
            "struct Pair{float values[65537];};void run(){Pair p=Pair();}",
            "T020",
        ),
        (
            "struct Pair{Texture2D<float> texture;};void run(){Pair p=Pair();}",
            "T020",
        ),
        (
            "struct S{float x=read(S());};float read(S s){return s.x;}void run(){S s=S();}",
            "I002",
        ),
    ] {
        let error = compile_source(
            "bad.slang",
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
    compile("struct S{float x=read(S());};float read(S s){return s.x;}void run(){}");
}
