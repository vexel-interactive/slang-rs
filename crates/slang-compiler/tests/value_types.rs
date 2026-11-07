use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn graph(levels: usize) -> String {
    let mut source = String::from("struct S0{float value;};");
    for index in 1..levels {
        source.push_str(&format!(
            "struct S{index}{{S{0} first;S{0} second[2];}};",
            index - 1
        ));
    }
    source
}

#[test]
fn shared_type_graphs_are_checked_without_expanding_all_paths() {
    let compiled = compile_source(
        "shared.slang",
        &graph(128),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(compiled.ir.structures.len(), 128);
}

#[test]
fn cycles_and_excessive_cached_depth_report_the_struct_source() {
    for source in [
        graph(129),
        "struct A{B b;};struct B{A a[2];};".into(),
        "struct A{float value;B child;};struct B{A a;};struct C{B child;};".into(),
    ] {
        let error = compile_source(
            "recursive.slang",
            &source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "T002"));
        assert!(
            error
                .diagnostics
                .iter()
                .filter(|d| d.code == "T002")
                .all(|d| d.span.end > d.span.start)
        );
    }
}
