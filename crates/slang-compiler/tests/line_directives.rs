use slang_compiler::{Compilation, CompileError, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str, files: &[(&str, &str)]) -> Result<Compilation, CompileError> {
    let includes = MemoryIncludes {
        files: files
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect(),
    };
    compile_source("root.slang", source, &CompileOptions::default(), &includes)
}

#[test]
fn zero_lines_fold_but_negative_line_tokens_are_invalid_typed_literals() {
    let compiled = compile("#line 0\nstatic const int line=__LINE__;\nint read(){return __LINE__;}\nint choose(int value){switch(value){case line:return 1;default:return 0;}}", &[]).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("line = 0;"), "{code}");
    assert!(code.contains("return 1;"), "{code}");
    for directive in ["4294967295", "2147483648"] {
        let error = compile(
            &format!("#line {directive}\nstatic const int line=__LINE__;"),
            &[],
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T010"),
            "{error}"
        );
    }
}

#[test]
fn remapped_constants_can_select_thread_group_dimensions() {
    let compiled = compile("#line 4 \"generated.slang\"\nstatic const int width=__LINE__;\n[shader(\"compute\")][numthreads(width,__LINE__,1)]void main(){}", &[]).unwrap();
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 5, 1])
    );
    assert!(compiled.artifact.unwrap().contains("[numthreads(4, 5, 1)]"));
}

#[test]
fn included_sources_remap_independently_and_resolve_from_physical_names() {
    let source = "#line 100 \"virtual/root.slang\"\n#include \"header.slang\"\nstatic const int rootLine=HEADER_LINE;\nint run(){return rootLine+includedLine;}";
    let compiled = compile(source, &[("header.slang", "#line 20 \"virtual/header.slang\"\n#include \"nested.slang\"\n#define HEADER_LINE __LINE__\nstatic const int includedLine=__LINE__;"), ("nested.slang", "#line 8 \"virtual/nested.slang\"\nstatic const int nestedLine=__LINE__;")]).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("nestedLine = 8;"), "{code}");
    assert!(code.contains("includedLine = 22;"), "{code}");
    assert!(code.contains("rootLine = 101;"), "{code}");
    assert_eq!(
        compiled
            .sources
            .files()
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["root.slang", "header.slang", "nested.slang"]
    );
}

#[test]
fn imports_and_member_files_keep_physical_resolution_and_logical_locations() {
    let compiled = compile("#line 90 \"virtual/root.slang\"\nimport lib;\nint run(){return read()+__LINE__;}", &[
        ("lib.slang", "#line 40 \"virtual/lib.slang\"\nmodule lib;\n__include \"member.slang\";\npublic int read(){return value()+__LINE__;}") ,
        ("member.slang", "#line 10 \"virtual/member.slang\"\nimplementing lib;\nint value(){return __LINE__;}") ,
    ]).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return 11;"), "{code}");
    assert!(code.contains("+ 42)"), "{code}");
    assert!(code.contains("+ 91)"), "{code}");
    assert_eq!(compiled.reflection.modules.len(), 2);
}

#[test]
fn lexer_parser_and_semantic_diagnostics_use_logical_coordinates_and_real_excerpts() {
    for (body, expected_line, excerpt) in [
        ("@", 70, "@"),
        ("float run( {", 70, "float run( {"),
        ("float run(){\nreturn missing;\n}", 71, "return missing;"),
    ] {
        let error = compile(&format!("#line 70 \"generated.slang\"\n{body}"), &[]).unwrap_err();
        let rendered = error.to_string();
        assert!(
            rendered.contains(&format!("generated.slang:{expected_line}:")),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!("{expected_line} | {excerpt}")),
            "{rendered}"
        );
        for diagnostic in &error.diagnostics {
            assert_eq!(
                error.sources.get(diagnostic.span.source).unwrap().name,
                "root.slang"
            );
        }
    }
}

#[test]
fn imported_diagnostics_preserve_logical_names_and_physical_source_ids() {
    let error = compile(
        "#line 60 \"virtual/root\"\nimport lib;\nint run(){return read();}",
        &[(
            "lib.slang",
            "#line 12 \"virtual/lib\"\nmodule lib;\npublic int read(){return missing;}",
        )],
    )
    .unwrap_err();
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "T011").unwrap();
    assert_eq!(
        error.sources.get(diagnostic.span.source).unwrap().name,
        "lib.slang"
    );
    let rendered = diagnostic.render(&error.sources);
    assert!(rendered.contains("virtual/lib:13:"), "{rendered}");
    assert!(
        rendered.contains("13 | public int read(){return missing;}"),
        "{rendered}"
    );
}

#[test]
fn profile_and_preprocessor_diagnostics_follow_line_mappings() {
    let options = CompileOptions {
        profile: Some("ps_6_0".parse().unwrap()),
        ..CompileOptions::default()
    };
    let error = compile_source("root.slang", "#line 50 \"generated.slang\"\n[shader(\"compute\")][numthreads(4,1,1)]void main(){float x=ddx(1.0);}", &options, &MemoryIncludes::default()).unwrap_err();
    assert!(error.to_string().contains("generated.slang:50:"), "{error}");
    let compiled = compile(
        "#line 20 \"generated.slang\"\n#warning check\nint run(){return 1;}",
        &[],
    )
    .unwrap();
    assert_eq!(compiled.diagnostics.len(), 1);
    assert!(
        compiled.diagnostics[0]
            .render(&compiled.sources)
            .contains("generated.slang:20:2")
    );
}

#[test]
fn default_and_empty_line_directives_reset_error_locations() {
    for reset in ["#line", "#line default"] {
        let error = compile(
            &format!("#line 50 \"generated\"\n{reset}\nint run(){{return missing;}}"),
            &[],
        )
        .unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("root.slang:3:"), "{rendered}");
        assert!(
            rendered.contains("3 | int run(){return missing;}"),
            "{rendered}"
        );
    }
}
