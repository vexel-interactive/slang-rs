use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "storage.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn local_statics_have_separate_storage_and_initialize_at_the_declaration() {
    let compiled = compile(include_str!("../../../examples/local-static-storage.slang"));
    assert_eq!(compiled.ir.globals.len(), 6);
    assert_eq!(compiled.reflection.globals.len(), 2);
    let delayed = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "delayed")
        .unwrap();
    let StmtKind::Block(body) = &delayed.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::If { then_branch, .. } = &body[0].kind else {
        panic!()
    };
    let StmtKind::If {
        condition,
        then_branch,
        ..
    } = &then_branch.kind
    else {
        panic!()
    };
    assert!(matches!(&condition.kind, ExprKind::Unary { op, .. } if op == "!"));
    let StmtKind::Block(initialize) = &then_branch.kind else {
        panic!()
    };
    let StmtKind::Expression(value) = &initialize[0].kind else {
        panic!()
    };
    let ExprKind::Binary { left, right, .. } = &value.kind else {
        panic!()
    };
    assert!(matches!(&right.kind, ExprKind::Call { name, .. } if name == "initialize"));
    let StmtKind::Return(Some(result)) = &body[1].kind else {
        panic!()
    };
    assert!(matches!((&result.kind, &left.kind), (ExprKind::Name(a), ExprKind::Name(b)) if a == b));
    assert!(compiled.artifact.unwrap().contains("= initialize(10));"));
}

#[test]
fn static_shadowing_groups_arrays_defaults_and_loop_initializers_keep_bindings() {
    let source = "static int value=7;struct P{int x=value;};int run(int seed){static int value=value+seed,next=value+1;static int items[2]={value,next};static P p=P();{int value=9;next+=value;}for(static int i=seed,j=i+1;i<3;i++)next+=j;switch(next){case 1:return items[0];default:return p.x+next;}}int other(){static const int value=3;switch(3){case value:return value;default:return 0;}}";
    let compiled = compile(source);
    assert_eq!(compiled.reflection.globals.len(), 1);
    let static_values: Vec<_> = compiled
        .ir
        .globals
        .iter()
        .filter(|g| g.name.contains("static_value_"))
        .collect();
    assert_eq!(static_values.len(), 7);
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let nested = body
        .iter()
        .find_map(|s| {
            if let StmtKind::Block(body) = &s.kind {
                Some(body)
            } else {
                None
            }
        })
        .unwrap();
    let StmtKind::Variable(local) = &nested[0].kind else {
        panic!()
    };
    assert!(matches!(&local.initializer.as_ref().unwrap().kind, ExprKind::Integer(v) if v == "9"));
    let StmtKind::Expression(add) = &nested[1].kind else {
        panic!()
    };
    assert!(
        matches!(&add.kind, ExprKind::Binary { right, .. } if matches!(&right.kind, ExprKind::Name(n) if n == &local.name))
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("= (value + seed));"));
    assert!(code.contains("case 3:"));
    assert!(!code.contains("for (static"));
}

#[test]
fn unbraced_declarations_keep_storage_outside_generated_control_blocks() {
    let compiled = compile(include_str!(
        "../../../examples/unbraced-declarations.slang"
    ));
    let main = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap();
    let StmtKind::Block(body) = &main.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Variable(value) = &body[0].kind else {
        panic!()
    };
    let StmtKind::Variable(next) = &body[1].kind else {
        panic!()
    };
    assert_eq!((&*value.name, &*next.name), ("value", "next"));
    assert!(value.initializer.is_none() && next.initializer.is_none());
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float value;\n    float next;\n    if"));
    assert!(code.contains("int iteration;\n    do"));
    assert!(code.contains("float temporary;\n        for"));
}

#[test]
fn a_single_static_loop_initializer_keeps_its_guard_before_the_loop() {
    let compiled =
        compile("int run(int seed){int sum=0;for(static int i=seed;i<3;i++)sum+=i;return sum;}");
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::For { initializer, .. } = &body[1].kind else {
        panic!()
    };
    let StmtKind::Sequence(statements) = &initializer.kind else {
        panic!()
    };
    assert!(matches!(statements[0].kind, StmtKind::If { .. }));
    let code = compiled.artifact.unwrap();
    let initializer = code.find("= seed)").unwrap();
    let loop_start = code.find("for (;").unwrap();
    assert!(initializer < loop_start);
}

#[test]
fn explicit_blocks_and_for_loops_bound_unbraced_bindings() {
    for (source, code) in [
        ("float run(){float a=1;if(true)float a=2;return a;}", "T003"),
        (
            "float run(){if(true)float a=1;else float a=2;return a;}",
            "T003",
        ),
        (
            "float run(){for(int i=0;i<1;i++)float i=1;return 0;}",
            "T003",
        ),
        (
            "float run(){for(int i=0;i<1;i++)float a=1;return a;}",
            "T011",
        ),
        ("float run(){if(true){float a=1;}return a;}", "T011"),
        ("float run(){while(a<1)int a=1;return a;}", "T011"),
        ("float run(){if(true)const float a=2;a=3;return a;}", "T015"),
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
    compile("static float a=2;float run(bool c){if(c)float a=a+1;else a=4;return a;}");
    compile(
        "float run(bool c){if(c)float values[2]={1,2};else values=float[2](3,4);return values[1];}",
    );
    compile("int run(){do const int value=2;while(value<0);return value;}");
    compile(
        "float run(bool c){if(c)const int label=2;switch(2){case label:return 1;default:return 0;}}",
    );
}

#[test]
fn imported_statics_and_generated_prefixes_keep_private_storage() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public int read(int input){static int value=input;return ++value;}".into(),
        )]
        .into(),
    };
    let compiled = compile_source("main.slang", "import lib;static int _slang_static_value_0=9;int readOwn(){static int value=1;return ++value;}int run(){return readOwn()+read(3)+_slang_static_value_0;}", &CompileOptions::default(), &includes).unwrap();
    assert_eq!(compiled.reflection.globals.len(), 1);
    assert_eq!(
        compiled
            .ir
            .globals
            .iter()
            .filter(|g| g.name.starts_with("_slang_static__value_"))
            .count(),
        2
    );
    assert!(compiled.artifact.unwrap().contains("= input)"));
}
