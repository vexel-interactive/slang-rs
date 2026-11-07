use slang_lexer::TokenKind;
use slang_preprocessor::{IncludeResolver, Options, Preprocessed, preprocess};
use slang_source::{SourceMap, Span};

struct NoIncludes;

impl IncludeResolver for NoIncludes {
    fn resolve(&self, _: &str, requested: &str, _: bool) -> Result<(String, String), String> {
        Err(format!("missing {requested}"))
    }
}

fn run(text: &str) -> (SourceMap, Preprocessed) {
    let mut sources = SourceMap::default();
    let root = sources.add("physical.slang", text);
    let result = preprocess(&mut sources, root, &NoIncludes, &Options::default());
    (sources, result)
}

fn values(result: &Preprocessed) -> Vec<&str> {
    result
        .tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .map(|t| t.text.as_str())
        .collect()
}

#[test]
fn remapped_arguments_retained_filenames_and_resets_follow_source_order() {
    let (_, result) = run(concat!(
        "#define NUMBER 50\n#define NAME \"virtual.slang\"\n#line NUMBER NAME\n",
        "__LINE__ __FILE__\n#line 10\n__LINE__ __FILE__\n",
        "#line default\n__LINE__ __FILE__\n#line 20 2\n__LINE__ __FILE__\n",
        "#line\n__LINE__ __FILE__\n"
    ));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(
        values(&result),
        [
            "50",
            "\"virtual.slang\"",
            "10",
            "\"virtual.slang\"",
            "8",
            "\"physical.slang\"",
            "20",
            "\"2\"",
            "12",
            "\"physical.slang\""
        ]
    );
}

#[test]
fn nested_and_multiline_macros_use_the_remapped_initiating_call() {
    for newline in ["\n", "\r\n", "\r"] {
        let text = concat!(
            "#define HERE __LINE__\n#define ID(x) x\n#define STR(x) #x\n#define EXPAND(x) STR(x)\n",
            "#line 80 \"virtual.slang\"\nHERE ID(\n__LINE__\n) __LINE__ EXPAND(__LINE__) STR(__LINE__)\n"
        ).replace('\n', newline);
        let (_, result) = run(&text);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            values(&result),
            ["80", "80", "82", "\"82\"", "\"__LINE__\""]
        );
    }
}

#[test]
fn signed_and_wrapped_line_numbers_match_upstream_conversion() {
    for (number, expected) in [
        ("0", "0"),
        ("4294967295", "-1"),
        ("2147483648", "-2147483648"),
        ("99999999999999999999999999999999", "-1"),
        ("0x10u", "16"),
        ("0X10", "0"),
        ("0b100", "0"),
        ("077", "77"),
        ("10ul", "10"),
    ] {
        let (_, result) = run(&format!("#line {number}\n__LINE__"));
        assert!(
            result.diagnostics.is_empty(),
            "{number}: {:?}",
            result.diagnostics
        );
        assert_eq!(values(&result), [expected], "{number}");
    }
    let (_, result) =
        run("#line 4294967295\n#line __LINE__\n__LINE__\n#if __LINE__ != 0\n#error wrong\n#endif");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(values(&result), ["-1"]);
}

#[test]
fn escaped_filenames_round_trip_through_file_macros_and_stringification() {
    let text = concat!(
        "#define STR(x) #x\n#define EXPAND(x) STR(x)\n",
        "#line 7 \"C:\\\\shader\\\\\\\"quoted\\\"\\n\\000f\\xCE\\xB1\\u03B2\\U0001F600\"\n",
        "__FILE__ EXPAND(__FILE__)\n#line 10 __FILE__\n__FILE__"
    );
    let (sources, result) = run(text);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let expected = r#""C:\\shader\\\"quoted\"\n\000fαβ😀""#;
    assert_eq!(values(&result)[0], expected);
    assert_eq!(values(&result)[2], expected);
    assert!(values(&result)[1].starts_with(r#""\"C:"#));
    let file = sources.get(result.tokens[0].span.source).unwrap();
    assert_eq!(
        file.presumed_location(result.tokens[0].span.start).name,
        "C:\\shader\\\"quoted\"\n\0fαβ😀"
    );
    let (_, result) = run(r#"#line 1 "\x{41}\u{3b1}\101"
__FILE__"#);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(values(&result), ["\"AαA\""]);
}

#[test]
fn invalid_directives_fail_without_panicking() {
    for args in [
        "-1",
        "1.5",
        "name",
        "10+2",
        "10 default",
        "10 \"name\" 1",
        "default 1",
        r#"10 "\q""#,
        r#"10 "\x""#,
        r#"10 "\uD800""#,
        r#"10 "\xFF""#,
        r#"10 "\777""#,
        r#"10 "\u{110000}""#,
    ] {
        let (_, result) = run(&format!("#line {args}\n__LINE__"));
        assert!(
            result.diagnostics.iter().any(|d| d.code == "P002"),
            "{args}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn continuations_and_trailing_tokens_use_keyword_line_adjustment() {
    let (sources, result) = run("#line 50 \\\n\"virtual.slang\"\n#error failure\n");
    assert_eq!(result.diagnostics.len(), 1);
    let rendered = result.diagnostics[0].render(&sources);
    assert!(rendered.contains("virtual.slang:51:2"), "{rendered}");
    assert!(rendered.contains("51 | #error failure"), "{rendered}");
    let (sources, result) = run("#line 30 \"virtual.slang\" 1\n__LINE__");
    assert_eq!(values(&result), ["30"]);
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        result.diagnostics[0]
            .render(&sources)
            .contains("virtual.slang:29:")
    );
}

#[test]
fn repeated_preprocessing_clears_mappings_from_previously_active_branches() {
    let text = "#ifdef REMAP\n#line 70 \"virtual\"\n#endif\n__LINE__ __FILE__";
    let mut sources = SourceMap::default();
    let root = sources.add("physical.slang", text);
    let options = Options {
        defines: [("REMAP".into(), "1".into())].into(),
        ..Options::default()
    };
    let first = preprocess(&mut sources, root, &NoIncludes, &options);
    assert!(first.diagnostics.is_empty());
    assert_eq!(values(&first), ["71", "\"virtual\""]);
    let second = preprocess(&mut sources, root, &NoIncludes, &Options::default());
    assert!(second.diagnostics.is_empty());
    assert_eq!(values(&second), ["4", "\"physical.slang\""]);
    let third = preprocess(&mut sources, root, &NoIncludes, &options);
    assert!(third.diagnostics.is_empty());
    assert_eq!(values(&third), values(&first));
    assert_eq!(
        third.tokens[0].span,
        Span::new(
            root,
            text.find("__LINE__").unwrap(),
            text.find("__LINE__").unwrap() + 8
        )
    );
}

#[test]
fn includes_receive_physical_origins_and_do_not_leak_logical_mappings() {
    struct PhysicalIncludes;
    impl IncludeResolver for PhysicalIncludes {
        fn resolve(
            &self,
            from: &str,
            requested: &str,
            _: bool,
        ) -> Result<(String, String), String> {
            match requested {
                "header" => {
                    assert_eq!(from, "physical/root");
                    Ok((
                        "physical/header".into(),
                        "#line 20 \"virtual/header\"\n#include \"nested\"\n__FILE__ __LINE__"
                            .into(),
                    ))
                }
                "nested" => {
                    assert_eq!(from, "physical/header");
                    Ok(("physical/nested".into(), "__FILE__ __LINE__".into()))
                }
                _ => Err("missing include".into()),
            }
        }
    }
    let mut sources = SourceMap::default();
    let root = sources.add(
        "physical/root",
        "#line 10 \"virtual/root\"\n#include \"header\"\n__FILE__ __LINE__",
    );
    let result = preprocess(&mut sources, root, &PhysicalIncludes, &Options::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(
        values(&result),
        [
            "\"physical/nested\"",
            "1",
            "\"virtual/header\"",
            "21",
            "\"virtual/root\"",
            "11"
        ]
    );
}
