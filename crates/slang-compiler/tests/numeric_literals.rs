use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, Severity, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "literals.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn suffixes_preserve_overload_types_and_valid_hlsl_spellings() {
    let code = compile(
        "float choose(half x){return 1;}float choose(float x){return 2;}float choose(double x){return 3;}float f(){return choose(1.5hf)+choose(1.5FH)+choose(1.5f)+choose(1.5LF)+choose(1.5fl);}",
    ).artifact.unwrap();
    assert_eq!(
        code.matches("choose__overload_0(((half)1.5f))").count(),
        2,
        "{code}"
    );
    assert!(code.contains("choose__overload_1(1.5f)"), "{code}");
    assert_eq!(
        code.matches("choose__overload_2(1.5L)").count(),
        2,
        "{code}"
    );
    let code = compile(
        "double f(){return 1e100lf;}half h(){return 1.0006hf;}float g(){return 0x1.000003p0;}",
    )
    .artifact
    .unwrap();
    assert!(
        !code.contains("1e100lf") && !code.contains("0x1."),
        "{code}"
    );
    assert!(code.contains("((half)1.0009766f)"), "{code}");
    assert!(code.contains("return 1.0000001f;"), "{code}");
}

#[test]
fn typed_literal_rounding_and_signed_literals_determine_dimensions() {
    for (expression, expected) in [
        ("int(1.99999999)", 2),
        ("int(1.99999999lf)", 1),
        ("int(2.9999hf)", 3),
        ("0x1.8p1", 3),
        ("int(-3.5)+5", 2),
        ("- -2.5", 2),
        ("float(2.5)", 2),
        ("double(2.5lf)", 2),
        ("(float)2.5", 2),
        ("+2.5", 2),
        ("-0.5", 0),
        ("!0.5", 1),
        ("bool(-0.5)", 0),
    ] {
        let compiled = compile(&format!("float data[{expression}];"));
        assert_eq!(
            compiled.reflection.globals[0].type_name,
            format!("float[{expected}]"),
            "{expression}"
        );
    }
    let compiled = compile("static const int Count=int(1.99999999);float data[Count];");
    assert_eq!(compiled.reflection.globals[1].type_name, "float[2]");
    let compiled = compile("static const int Count=2L;float data[Count];uint f(){return 3Lu;}");
    assert_eq!(compiled.reflection.globals[1].type_name, "float[2]");
    assert!(compiled.artifact.unwrap().contains("return 3u;"));
}

#[test]
fn range_and_precision_warnings_keep_the_converted_literal_values() {
    for (text, warning, emitted) in [
        ("65520.0h", "T028", "((half)asfloat(0x7f800000u))"),
        ("1e50", "T028", "asfloat(0x7f800000u)"),
        ("1e9999L", "T028", "asdouble(0u, 0x7ff00000u)"),
        ("1e-9999", "T027", "0.0f"),
        ("0x1p-150", "T027", "0.0f"),
        ("0x1.000003p0", "T029", "1.0000001f"),
    ] {
        let compiled = compile(&format!("double f(){{return {text};}}"));
        assert!(
            compiled
                .diagnostics
                .iter()
                .any(|d| d.code == warning && d.severity == Severity::Warning),
            "{text}: {:?}",
            compiled.diagnostics
        );
        assert!(compiled.artifact.unwrap().contains(emitted), "{text}");
    }
    for text in ["0#INF", "1#INFf", "1.0#INFh", "1#INFL"] {
        assert!(
            compile(&format!("double f(){{return {text};}}"))
                .diagnostics
                .is_empty(),
            "{text}"
        );
    }
}

#[test]
fn octal_values_and_preprocessor_numbers_follow_their_distinct_rules() {
    let compiled = compile(
        "#if 010 != 10 || 0b10 != 0 || 0x10 != 16 || 0X10 != 0 || 4294967295u != -1\n#error numeric base\n#endif\nstatic const int Count=010;float data[Count];int f(){return 010;}",
    );
    assert_eq!(compiled.reflection.globals[1].type_name, "float[8]");
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("static const int Count = 8;")
    );
    assert!(compiled.diagnostics.iter().any(|d| d.code == "L005"));
}

#[test]
fn malformed_literals_and_general_floating_dimensions_remain_diagnostics() {
    for (source, code) in [
        ("float f(){return 2f;}", "L004"),
        ("half f(){return 1.0hF;}", "L004"),
        ("double f(){return 1.0lF;}", "L004"),
        ("float f(){return 0x1p+;}", "L004"),
        ("int f(){return 09;}", "L004"),
        ("float data[float(2)];", "T002"),
        ("static const float Count=2.5;float data[Count];", "T002"),
        ("float data[1.5+1.5];", "T002"),
        ("float data[-(-2.5)];", "T002"),
        ("float data[int((2.5))];", "T002"),
        ("float data[-float(2.5)];", "T002"),
        ("float data[+(float)2.5];", "T002"),
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
