use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_source::Severity;

#[test]
fn unknown_pragmas_warn_and_compilation_continues() {
    let compiled = compile_source(
        "pragmas.slang",
        r#"#define NAME once
#define BAD(a,b) a+b
#pragma NAME
#pragma custom BAD(1)

int run()
{
    return 7;
}
"#,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();

    assert!(compiled.artifact.unwrap().contains("return 7;"));
    assert_eq!(compiled.diagnostics.len(), 2);

    for (diagnostic, line) in compiled.diagnostics.iter().zip([3, 4]) {
        assert_eq!(diagnostic.code, "P011");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(
            compiled.sources.get(diagnostic.span.source).unwrap().location(diagnostic.span.start),
            (line, 9)
        );
    }
}

#[test]
fn unknown_pragma_warnings_survive_included_and_imported_sources() {
    let includes = MemoryIncludes {
        files: [
            (
                "header.slang".into(),
                "#pragma once\n#pragma header_specific raw(payload)\nstatic const int value = 3;\n"
                    .into(),
            ),
            (
                "library.slang".into(),
                r#"#pragma library_specific
module library;

public int helper()
{
    return 2;
}
"#
                .into(),
            ),
        ]
        .into(),
    };
    let compiled = compile_source(
        "pragmas.slang",
        r#"#include "header.slang"
#include "header.slang"
import library;

int run()
{
    return value + helper();
}
"#,
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();

    assert_eq!(compiled.diagnostics.len(), 2);

    for (name, line) in [("header.slang", 2), ("library.slang", 1)] {
        let diagnostic = compiled
            .diagnostics
            .iter()
            .find(|d| compiled.sources.get(d.span.source).unwrap().name == name)
            .unwrap();

        assert_eq!(diagnostic.code, "P011");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(
            compiled.sources.get(diagnostic.span.source).unwrap().location(diagnostic.span.start),
            (line, 9)
        );
    }

    assert_eq!(compiled.reflection.modules.len(), 2);
    assert_eq!(compiled.ir.globals.iter().filter(|g| g.name == "value").count(), 1);
}

#[test]
fn malformed_and_unimplemented_known_pragmas_remain_errors() {
    for (pragma, column) in [
        ("#pragma", 2),
        ("#pragma 1", 2),
        ("#pragma once trailing", 14),
        ("#pragma warning(push)", 9),
    ] {
        let error = compile_source(
            "pragmas.slang",
            &format!("{pragma}\n\nint run()\n{{\n    return 1;\n}}\n"),
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();

        assert_eq!(error.diagnostics.len(), 1);
        let diagnostic = &error.diagnostics[0];

        assert_eq!(diagnostic.code, "P002");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(
            error.sources.get(diagnostic.span.source).unwrap().location(diagnostic.span.start),
            (1, column)
        );
    }
}
