use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "comma.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn comma_sequences_void_calls_and_keeps_the_right_value_type() {
    let compiled = compile("void tick(){}float2 run(){return tick(),float2(1,2);}");
    let function = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &function.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(value)) = &body[0].kind else {
        panic!()
    };
    assert_eq!(value.ty, Type::Vector(Scalar::Float, 2));
    let ExprKind::Binary { op, left, right } = &value.kind else {
        panic!()
    };
    assert_eq!(op, ",");
    assert_eq!(left.ty, Type::Void);
    assert_eq!(right.ty, value.ty);
}

#[test]
fn comma_precedence_preserves_argument_and_default_separators() {
    let compiled = compile(
        "float read(float a,float b=3){return a+b;}float run(){float x=1;float y=2;for(x=0,y=1;x<3;++x,++y){}return read((x=2,x+1),4)+read((y=3,y));}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((x = 0) , (y = 1))"));
    assert!(code.contains("((++x) , (++y))"));
    assert!(code.contains("read(((x = 2) , (x + 1)), 4)"));
}

#[test]
fn comma_results_are_values_and_are_not_supported_integer_constants() {
    for (source, code) in [
        ("void run(){float a=1;float b=2;(a,b)=3;}", "T015"),
        (
            "void write(out float x){x=1;}void run(){float a=1;float b=2;write((a,b));}",
            "T015",
        ),
        (
            "float run(){switch(2){case (1,2):return 3;default:return 4;}}",
            "T017",
        ),
        (
            "struct P{float x;float y;};void run(){P p=(1,{2,3});}",
            "T008",
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
}
