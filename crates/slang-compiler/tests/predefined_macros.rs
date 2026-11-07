use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn default_slang_define_can_be_overridden_by_the_caller() {
    let source = "#if __SLANG__\nfloat run(){return 1;}\n#else\nfloat run(){return 2;}\n#endif";
    let default = compile_source(
        "root.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(default.artifact.unwrap().contains("return 1;"));
    let options = CompileOptions {
        defines: [("__SLANG__".into(), "0".into())].into(),
        ..CompileOptions::default()
    };
    let overridden =
        compile_source("root.slang", source, &options, &MemoryIncludes::default()).unwrap();
    assert!(overridden.artifact.unwrap().contains("return 2;"));
}

#[test]
fn included_macro_definitions_use_root_invocations_and_local_header_lines() {
    let files = MemoryIncludes {
        files: [(
            "header.slang".into(),
            "#define HEADER_LINE __LINE__\nstatic const int includedLine=__LINE__;\n".into(),
        )]
        .into(),
    };
    let compiled = compile_source("root.slang", "#include \"header.slang\"\nstatic const int rootLine=HEADER_LINE;\nint run(){return includedLine+rootLine;}\n", &CompileOptions::default(), &files).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("includedLine = 2;"));
    assert!(code.contains("rootLine = 2;"));
}

#[test]
fn imported_modules_get_fresh_location_builtins_and_the_slang_define() {
    let files = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;\n#if __SLANG__\npublic int line(){return __LINE__;}\n#endif\n".into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "root.slang",
        "import lib;\nint run(){return line()+__LINE__;}\n",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return 3;"));
    assert!(code.contains("+ 2)"));
}

#[test]
fn builtin_redefinition_warnings_keep_original_source_locations() {
    let compiled = compile_source(
        "root.slang",
        "#define __LINE__ 7\nint run(){return __LINE__;}\n",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(compiled.artifact.unwrap().contains("return 7;"));
    assert_eq!(compiled.diagnostics.len(), 1);
    let diagnostic = &compiled.diagnostics[0];
    assert_eq!(diagnostic.code, "P010");
    assert_eq!(
        compiled
            .sources
            .get(diagnostic.span.source)
            .unwrap()
            .location(diagnostic.span.start),
        (1, 9)
    );
}

#[test]
fn file_macro_can_request_a_pragma_once_source() {
    let source = "#pragma once\n#include __FILE__\nfloat run(){return 1;}\n";
    let files = MemoryIncludes {
        files: [("root.slang".into(), source.into())].into(),
    };
    let compiled =
        compile_source("root.slang", source, &CompileOptions::default(), &files).unwrap();
    assert_eq!(compiled.ir.functions.len(), 1);
}
