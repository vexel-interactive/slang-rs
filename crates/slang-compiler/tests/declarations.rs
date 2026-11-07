use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "declarations.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn declarators_keep_independent_arrays_and_sequential_local_bindings() {
    let compiled = compile(
        "float run(){float a[2]={1,2},b=a[1]+3,c[3]={b,5,6};float[2] row={1,2},grid[3]={{3,4},{5,6},{7,8}};return a[0]+b+c[2]+grid[2][1];}",
    );
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Variables(values) = &body[0].kind else {
        panic!()
    };
    let types: Vec<_> = values.iter().map(|v| v.ty.to_string()).collect();
    assert_eq!(types, ["float[2]", "float", "float[3]"]);
    let ExprKind::Binary { left, .. } = &values[1].initializer.as_ref().unwrap().kind else {
        panic!()
    };
    assert!(
        matches!(&left.kind, ExprKind::Index { base, .. } if matches!(&base.kind, ExprKind::Name(n) if n == "a"))
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float grid[3][2]"));
}

#[test]
fn grouped_globals_fields_and_const_bindings_keep_individual_metadata() {
    let compiled = compile(
        "static const int first=1,second=first+1;struct P{float x=2,y=3,values[2]={4,5};};float run(){const int a=1,b=a+1;P p=P();switch(second){case b:return p.y+p.values[1];default:return p.x;}}",
    );
    let globals: Vec<_> = compiled
        .ir
        .globals
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(globals, ["first", "second"]);
    let fields: Vec<_> = compiled.ir.structures[0]
        .fields
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(fields, ["x", "y", "values"]);
    assert!(
        compiled
            .ir
            .globals
            .iter()
            .all(|v| v.modifiers.iter().any(|m| m == "const"))
    );
}

#[test]
fn grouped_for_initializers_share_loop_scope_without_leaking_bindings() {
    let compiled = compile(
        "float run(){float sum=0;for(int i=0,j=i+2;i<3;i++,j++){if(j==3)continue;sum+=j;}float j=5;return sum+j;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("int j = (i + 2);"));
    assert!(code.contains("for (; (i < 3); ((i++) , (j++)))"));
    assert!(code.contains("float j = 5;"));
}

#[test]
fn grouped_imported_declarations_preserve_visibility_and_local_shadowing() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public static float value=1,other=2;public struct P{public float x=3,y=4;};public float read(){float value=5,next=value+1;return next+other;}".into(),
        )].into(),
    };
    let compiled = compile_source(
        "main.slang",
        "import lib;float run(){float value=7,next=value+1;P p=P();return next+read()+other+p.y;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("float next = (value + 1);").count(), 2);
    assert!(code.contains("_slang__slang_module_1_other"));
}

#[test]
fn invalid_groups_report_member_errors_and_keep_loop_scope() {
    for (source, code) in [
        ("float run(){float a=1,a=2;return a;}", "T003"),
        ("struct P{float x=1,x=2;};", "T003"),
        ("static float a=1,a=2;", "T003"),
        ("float run(){float a=missing,b=2;return b;}", "T011"),
        ("float run(){float a=b,b=2;return a;}", "T011"),
        ("float run(){float a=1,b=float2(2);return a;}", "T008"),
        ("float run(){const float a=1,b;return a;}", "T004"),
        ("float run(){const float a=1,b=2;b=3;return a;}", "T015"),
        (
            "float run(){for(int i=0,j=2;i<3;i++,j++){}return j;}",
            "T011",
        ),
        ("float run(){float a=1,;return a;}", "S001"),
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
