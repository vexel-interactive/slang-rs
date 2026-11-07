use slang_compiler::*;

#[test]
fn compute_pipeline_and_reflection() {
    let source = include_str!("../../../examples/vector-add.slang");
    let compilation = compile_source(
        "vector-add.slang",
        source,
        &CompileOptions {
            entry_point: Some("main".into()),
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    let code = compilation.artifact.unwrap();
    assert!(code.contains("[numthreads(64, 1, 1)]"));
    assert!(code.contains("RWStructuredBuffer<float> result : register(u0);"));
    assert!(code.contains("(result)[(id).x] = ((left)[(id).x] + (right)[(id).x])"));
    assert_eq!(compilation.reflection.globals.len(), 3);
    assert_eq!(compilation.reflection.entry_points[0].name, "main");
    let ir = serde_json::to_value(&compilation.ir).unwrap();
    assert!(ir["functions"][0]["body"]["kind"]["Block"][0]["kind"]["Expression"]["ty"].is_object());
}

#[test]
fn includes_once_and_guards() {
    let mut includes = MemoryIncludes::default();
    includes.files.insert(
        "constants.slang".into(),
        "#pragma once\nstatic const float factor = 2.0;\n".into(),
    );
    includes.files.insert("guarded.slang".into(), "#ifndef GUARDED\n#define GUARDED\n#include \"guarded.slang\"\nfloat twice(float x) {return x*factor;}\n#endif\n".into());
    let source = "#include \"constants.slang\"\n#include \"constants.slang\"\n#include \"guarded.slang\"\nfloat f(){return twice(1.0);}";
    let compilation =
        compile_source("test", source, &CompileOptions::default(), &includes).unwrap();
    assert_eq!(compilation.ir.globals.len(), 1);
    assert_eq!(compilation.ir.functions.len(), 2);
}

#[test]
fn unsupported_targets_fail_and_check_only_has_no_artifact() {
    let options = CompileOptions {
        target: Target::Spirv,
        ..CompileOptions::default()
    };
    let error =
        compile_source("test", "void f(){}", &options, &MemoryIncludes::default()).unwrap_err();
    assert_eq!(error.diagnostics[0].code, "G001");
    let options = CompileOptions {
        check_only: true,
        ..options
    };
    assert!(
        compile_source("test", "void f(){}", &options, &MemoryIncludes::default())
            .unwrap()
            .artifact
            .is_none()
    );
}

#[test]
fn errors_keep_the_include_source_location() {
    let includes = MemoryIncludes {
        files: [("bad.slang".into(), "void f(){missing=1;}".into())].into(),
    };
    let error = compile_source(
        "root.slang",
        "#include \"bad.slang\"",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(error.to_string().contains("bad.slang:1:"));
}

#[test]
fn compilation_sessions_do_not_share_macros() {
    let first = CompileOptions {
        defines: [("VALUE".into(), "1".into())].into(),
        ..CompileOptions::default()
    };
    compile_source(
        "a",
        "int f(){return VALUE;}",
        &first,
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(
        compile_source(
            "b",
            "int f(){return VALUE;}",
            &CompileOptions::default(),
            &MemoryIncludes::default()
        )
        .is_err()
    );
}
