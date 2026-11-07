use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn trailing_defaults_expand_each_omission_and_keep_full_signatures() {
    let compiled = compile(
        "static float counter=0;float next(){counter+=1;return counter;}float read(float x=next(),float y=2){return x+y;}float run(){return read()+read(3)+read(3,4);}",
    );
    let read = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "read")
        .unwrap();
    assert_eq!(read.parameters.len(), 2);
    assert!(read.parameters.iter().all(|p| p.initializer.is_none()));
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("next()").count(), 3);
    assert!(!code.contains("float x ="));
}

#[test]
fn default_helpers_preserve_declaration_scope_under_local_shadowing() {
    let compiled = compile(
        "static float value=2;float helper(){return value;}float read(float x=helper()+value){return x;}float run(){float value=9;float helper=8;float _slang_init_default_0=7;return read()+value+helper;}",
    );
    let helper = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name.contains("default_") && f.name != "run")
        .unwrap();
    assert!(helper.parameters.is_empty());
    let StmtKind::Block(body) = &helper.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(value)) = &body[0].kind else {
        panic!()
    };
    assert!(matches!(value.kind, ExprKind::Binary { .. }));
    assert_ne!(helper.name, "_slang_init_default_0");
    let code = compiled.artifact.unwrap();
    assert!(code.contains("helper() + value"));
}

#[test]
fn prototype_defaults_use_the_first_declaration() {
    let compiled = compile(
        "float read(float x=2);float read(float x=3){return x;}float other(float x);float other(float x=4){return x;}float run(){return read()+other(5);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return ((float)2)"));
    assert!(!code.contains("return ((float)4)"));
    assert!(!code.contains("return ((float)3)"));
}

#[test]
fn default_arrays_structs_and_imports_keep_context_and_ownership() {
    let compiled = compile(
        "struct P{float x=7;};float read(P p={},float a[2]={1,2}){return p.x+a[1];}float run(){return read();}",
    );
    assert!(compiled.artifact.unwrap().contains("default_result"));
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;float value(){return 3;}public float read(float x=value()){return x;}"
                .into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "main.slang",
        "import lib;float value(){return 9;}float run(){return read();}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("_slang__slang_module_1_value()")
    );
}

#[test]
fn overloads_rank_supplied_arguments_without_preferring_fewer_defaults() {
    compile(
        "float read(float x,int y=3){return x+y;}float read(int x,int y=4){return x+y;}float run(){return read(2);}",
    );
    for (source, code) in [
        (
            "float read(float x){return x;}float read(float x,float y=3){return x+y;}float run(){return read(2);}",
            "T014",
        ),
        (
            "float read(float x);float read(float x=2){return x;}float run(){return read();}",
            "T014",
        ),
        ("float read(float x=1,float y){return x+y;}", "T022"),
        ("void write(out float x=1){x=2;}", "T022"),
        ("float read(float x,float y=x){return y;}", "T022"),
        ("float read(float2 x=float3(1)){return x.x;}", "T008"),
        (
            "float read(float x=read()){return x;}float run(){return read();}",
            "I002",
        ),
        (
            "float read(const int x=2){switch(x){case x:return 1;default:return 2;}}",
            "T017",
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
