use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn switch_fallthrough_do_while_and_casts_lower_together() {
    let compiled = compile_source(
        "switch-loop.slang",
        include_str!("../../../examples/switch-loop.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("case special:"));
    assert!(code.contains("while ((i < 4u));"));
    assert!(code.contains("((int)(values).x)"));
}

#[test]
fn break_paths_do_not_count_as_return_paths() {
    for source in [
        "int f(int x,bool stop){switch(x){case 0:if(stop)break;default:return 1;}}",
        "int f(int x){switch(x){case 0:break;return 1;default:return 2;}}",
        "int f(int x,bool stop){do {if(stop)break;return 1;}while(x);}",
    ] {
        let error = compile_source(
            "flow.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T005"),
            "{source}: {error}"
        );
    }
    for source in [
        "int f(int x){switch(x){case 0:case 1:return 1;default:return 2;}}",
        "int f(bool x){do {return 1;}while(x);}",
        "int f(int x,bool stop){switch(x){case 0:if(stop)break;default:return 1;}return 2;}",
        "int f(int x,bool stop){if(stop)return 1;return 2;}",
    ] {
        assert!(
            compile_source(
                "flow.slang",
                source,
                &CompileOptions::default(),
                &MemoryIncludes::default()
            )
            .is_ok(),
            "{source}"
        );
    }
}

#[test]
fn switch_constants_follow_integer_types_and_scopes() {
    for source in [
        "void f(uint x){switch(x){case -1:break;case 0xffffffffu:break;}}",
        "void f(int x){const int value=1<<2;switch(x){case value:break;case 4:break;}}",
        "void f(int x){switch(x){case (-1>>1u):break;case -1:break;}}",
        "void f(int x){switch(x){case (int)2.5:break;case 2:break;}}",
    ] {
        let error = compile_source(
            "constant.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T017"),
            "{source}: {error}"
        );
    }
    let source = "static const int label=3;void f(int x){const int label=4;switch(x){case label:break;}}void g(int x){switch(x){case label:break;}}";
    assert!(
        compile_source(
            "scopes.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default()
        )
        .is_ok()
    );
}

#[test]
fn parenthesized_values_and_casts_keep_distinct_grammar() {
    for source in [
        "float f(float x){return (x)+1.0;}",
        "float f(int x){return (float)x+1.0;}",
        "bool f(int x){return (bool)x;}",
    ] {
        assert!(
            compile_source(
                "cast.slang",
                source,
                &CompileOptions::default(),
                &MemoryIncludes::default()
            )
            .is_ok(),
            "{source}"
        );
    }
}
