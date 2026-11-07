use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "bindings.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn a_shadowing_initializer_reads_the_previous_global_binding() {
    let compiled = compile("static float value=2;float run(){float value=value+1;return value;}");
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Variable(local) = &body[0].kind else {
        panic!()
    };
    let ExprKind::Binary { left, .. } = &local.initializer.as_ref().unwrap().kind else {
        panic!()
    };
    assert!(matches!(&left.kind, ExprKind::Name(n) if n == "value"));
    assert_ne!(local.name, "value");
    let StmtKind::Return(Some(value)) = &body[1].kind else {
        panic!()
    };
    assert!(matches!(&value.kind, ExprKind::Name(n) if n == &local.name));
    assert_eq!(compiled.reflection.globals[0].name, "value");
    assert_eq!(compiled.reflection.globals[0].target_name, "value");
}

#[test]
fn nested_parameter_and_group_bindings_keep_initializer_identity() {
    for source in [
        "float run(float value){{float value=value+1,next=value+1;return next;}}",
        "float run(){float value=2;{float value=value+1,next=value+1;return next;}}",
    ] {
        let compiled = compile(source);
        let run = compiled
            .ir
            .functions
            .iter()
            .find(|f| f.name == "run")
            .unwrap();
        let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
            panic!()
        };
        let outer = if run.parameters.is_empty() {
            let StmtKind::Variable(outer) = &body[0].kind else {
                panic!()
            };
            &outer.name
        } else {
            &run.parameters[0].name
        };
        let StmtKind::Block(inner) = &body.last().unwrap().kind else {
            panic!()
        };
        let StmtKind::Variables(locals) = &inner[0].kind else {
            panic!()
        };
        let ExprKind::Binary { left, .. } = &locals[0].initializer.as_ref().unwrap().kind else {
            panic!()
        };
        assert!(matches!(&left.kind, ExprKind::Name(n) if n == outer));
        assert_ne!(locals[0].name, *outer);
        let ExprKind::Binary { left, .. } = &locals[1].initializer.as_ref().unwrap().kind else {
            panic!()
        };
        assert!(matches!(&left.kind, ExprKind::Name(n) if n == &locals[0].name));
    }
}

#[test]
fn renamed_const_bindings_and_generated_prefixes_keep_source_diagnostics() {
    compile(
        "static const int value=1;int run(){const int value=value+1;switch(2){case value:return value;default:return 0;}}",
    );
    let compiled = compile(
        "static float value=2;float run(){float _slang_module_local_0_0=7;float value=value+1;return value+_slang_module_local_0_0;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("_slang__slang_module__local_0_"));
    let error = compile_source(
        "bad.slang",
        "static float value=2;float run(){float value=3,value=4;return value;}",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "T003" && d.message == "duplicate variable 'value'")
    );
}
