use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn character_values_are_uint_in_overloads_and_constructors() {
    let compiled = compile_source("characters.slang",
        r"float choose(int value){return 1;}float choose(uint value){return 2;}float f(){uint2 values=uint2('A','\u{ffffffff}');return choose('A')+values.x;}",
        &CompileOptions::default(), &MemoryIncludes::default()).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("choose(65u)"), "{code}");
    assert!(code.contains("uint2(65u, 4294967295u)"), "{code}");
    assert!(compiled.diagnostics.is_empty());
}

#[test]
fn characters_fold_in_macros_dimensions_forward_constants_and_switches() {
    let compiled = compile_source("characters.slang",
        "#define LETTER '\\x41'\nstatic const uint Count=Last-LETTER;static const uint Last='C';float data[Count];uint f(uint value){switch(value){case 'A':return LETTER;case '\\u0042':return '\\777';default:return '\\Uffffffff';}}",
        &CompileOptions::default(), &MemoryIncludes::default()).unwrap();
    assert_eq!(compiled.reflection.globals[2].type_name, "float[2]");
    let code = compiled.artifact.unwrap();
    assert!(code.contains("case 65u:"), "{code}");
    assert!(code.contains("return 511u;"), "{code}");
    assert!(code.contains("return 4294967295u;"), "{code}");
}

#[test]
fn imported_character_alias_dimensions_and_defaults_keep_values() {
    let compiled = compile_source("main.slang",
        "import chars;uint run(){Code values=Code('A','B');return values[0]+pick();}",
        &CompileOptions::default(),
        &MemoryIncludes { files: [("chars.slang".into(), "module chars;public typedef uint Code['C'-'A'];public uint pick(uint value='A'){return value;}".into())].into() }).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("values[2]"), "{code}");
    assert!(code.contains("return 65u;"), "{code}");
}

#[test]
fn malformed_character_values_are_diagnosed_at_the_literal_span() {
    for literal in [
        "''",
        "'ab'",
        r"'\q'",
        r"'\x'",
        r"'\u12'",
        r"'\U1234'",
        r"'\x{}'",
        r"'\u{12'",
        r"'\x100000000'",
    ] {
        let source = format!("uint f(){{return {literal};}}");
        let error = compile_source(
            "bad.slang",
            &source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        let diagnostic = error
            .diagnostics
            .iter()
            .find(|d| d.code == "L006")
            .unwrap_or_else(|| panic!("{literal}: {error}"));
        assert_eq!(&source[diagnostic.span.start..diagnostic.span.end], literal);
    }
}
