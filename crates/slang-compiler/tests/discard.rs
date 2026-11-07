use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::StmtKind;

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "discard.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn fails(source: &str, code: &str) {
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

#[test]
fn fragment_discard_and_numeric_clip_lower_to_native_hlsl() {
    let compiled = compile(
        "void kill(){discard;}[shader(\"fragment\")]float4 main(float2 uv:TEXCOORD0):SV_Target0{if(uv.x<0.5)kill();clip(uv);clip(int2(-1,2));clip(float2x2(1));return float4(1);}",
    );
    let kill = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "kill")
        .unwrap();
    let StmtKind::Block(body) = &kill.body.as_ref().unwrap().kind else {
        panic!()
    };
    assert!(matches!(body[0].kind, StmtKind::Discard));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("discard;"));
    assert!(code.contains("clip(uv)"));
}

#[test]
fn selected_overloads_and_user_clip_names_do_not_inherit_unused_requirements() {
    compile(
        "void kill(float x){discard;}void kill(int x){}void clip(float x){}[shader(\"compute\")][numthreads(1,1,1)]void main(){kill(1);clip(2.0);}",
    );
    compile(
        "struct clip{float x;};[shader(\"compute\")][numthreads(1,1,1)]void main(){clip p=clip(2);}",
    );
}

#[test]
fn supplied_arguments_bypass_fragment_defaults() {
    compile(
        "float read(){discard;return 1;}float sample(float x=read()){return x;}struct P{float x=read();};[shader(\"compute\")][numthreads(1,1,1)]void main(){float v=sample(2);P p=P(3);}",
    );
}

#[test]
fn stage_requirements_reach_calls_defaults_fields_and_used_globals() {
    let entry = "[shader(\"compute\")][numthreads(1,1,1)]void main()";
    for source in [
        format!("{entry}{{discard;}}"),
        format!("{entry}{{clip(-1.0);}}"),
        format!("void kill(){{discard;}}void relay(){{kill();}}{entry}{{relay();}}"),
        format!(
            "float read(){{discard;return 1;}}float sample(float x=read()){{return x;}}{entry}{{float v=sample();}}"
        ),
        format!(
            "float read(){{discard;return 1;}}struct P{{float x=read();}};struct Q{{P p[2];}};{entry}{{Q q={{}};}}"
        ),
        format!(
            "float read(){{discard;return 1;}}static float value=read();{entry}{{float v=value;}}"
        ),
        "[shader(\"vertex\")]float4 main():SV_Position{discard;return float4(0);}".into(),
    ] {
        fails(&source, "T023");
    }
}

#[test]
fn imported_and_cyclic_call_graphs_propagate_without_stack_recursion() {
    let files = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;void kill(){discard;}public void relay(){kill();}".into(),
        )]
        .into(),
    };
    let error = compile_source(
        "main.slang",
        "import lib;[shader(\"compute\")][numthreads(1,1,1)]void main(){relay();}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap_err();
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "T023").unwrap();
    assert!(
        error
            .sources
            .get(diagnostic.span.source)
            .unwrap()
            .name
            .ends_with("lib.slang")
    );
    let mut source = "void f0(){discard;}".to_owned();
    for i in 1..256 {
        source.push_str(&format!("void f{i}(){{f{}();}}", i - 1));
    }
    source.push_str("[shader(\"compute\")][numthreads(1,1,1)]void main(){f255();}");
    fails(&source, "T023");
    fails(
        "void a(){b();}void b(){a();discard;}[shader(\"compute\")][numthreads(1,1,1)]void main(){a();}",
        "T023",
    );
}

#[test]
fn invalid_clip_calls_and_missing_returns_remain_diagnostics() {
    fails("void run(){clip(true);}", "T014");
    fails("void run(){clip();}", "T014");
    fails("void run(){clip(1,2);}", "T014");
    fails("void run(){discard}", "S001");
    fails("float run(){discard;}", "T005");
}
