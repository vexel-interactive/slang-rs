use std::{fs, process::Command};

#[test]
fn cli_reports_deep_syntax_without_exhausting_its_stack() {
    let root = std::env::temp_dir().join(format!("slang-rs-cli-depth-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("deep.slang");
    for source in [
        format!(
            "int f(){{return {}1{};}}",
            "(".repeat(1000),
            ")".repeat(1000)
        ),
        format!("int f(){{return {}1;}}", "(int)".repeat(1000)),
        format!(
            "void f(){{float a={}1{};}}",
            "{".repeat(1000),
            "}".repeat(1000)
        ),
        format!("void f(){{{};}}", "if(true)".repeat(1000)),
        format!("void f(){{{}{}}}", "{".repeat(1000), "}".repeat(1000)),
        format!("void f(){{{};}}", "for(;;)".repeat(1000)),
        format!(
            "void f(){{{};{}}}",
            "do ".repeat(1000),
            "while(true);".repeat(1000)
        ),
        format!(
            "void f(){{{};{}}}",
            "switch(0){default:".repeat(1000),
            "}".repeat(1000)
        ),
    ] {
        fs::write(&input, source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(&input)
            .arg("--check")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("S001"));
    }
    fs::remove_file(&input).unwrap();
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_rejects_output_aliases_of_sources() {
    let root = std::env::temp_dir().join(format!("slang-rs-cli-alias-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("valid.slang");
    let included = root.join("included.slang");
    let source = "#include \"included.slang\"\nvoid f(){}";
    fs::write(&input, source).unwrap();
    fs::write(&included, "static const int value=1;").unwrap();
    let input_alias = root
        .join("..")
        .join(root.file_name().unwrap())
        .join("valid.slang");
    for output in [input_alias, included.clone()] {
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(&input)
            .arg("-o")
            .arg(output)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("overwrite"));
    }
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    assert_eq!(
        fs::read_to_string(&included).unwrap(),
        "static const int value=1;"
    );
    fs::remove_file(&input).unwrap();
    fs::remove_file(&included).unwrap();
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_preserves_existing_output_on_compile_failure() {
    let root = std::env::temp_dir().join(format!("slang-rs-cli-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("invalid.slang");
    let output = root.join("shader.hlsl");
    fs::write(&input, "void f(){unknown=1;}").unwrap();
    fs::write(&output, "previous artifact").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "previous artifact");
    assert!(String::from_utf8_lossy(&result.stderr).contains("T011"));
    for path in [&input, &output] {
        fs::remove_file(path).unwrap();
    }
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_reports_excessive_module_graph_depth() {
    let root = std::env::temp_dir().join(format!("slang-rs-module-depth-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for member in [false, true] {
        for index in 0..40 {
            let directive = if member { "__include" } else { "import" };
            let declaration = if member && index > 0 {
                "implementing m0;".into()
            } else {
                format!("module m{index};")
            };
            fs::write(
                root.join(format!("m{index}.slang")),
                format!("{declaration}{directive} m{};", index + 1),
            )
            .unwrap();
        }
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(root.join("m0.slang"))
            .arg("--check")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("M002"));
    }
    for index in 0..40 {
        fs::remove_file(root.join(format!("m{index}.slang"))).unwrap();
    }
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_reports_excessive_alias_and_array_nesting() {
    let root = std::env::temp_dir().join(format!("slang-rs-alias-depth-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("alias.slang");
    let mut aliases = String::new();
    for index in 0..100 {
        aliases.push_str(&format!("typealias T{index}=T{};", index + 1));
    }
    aliases.push_str("typealias T100=float;");
    for (source, code) in [
        (aliases, "T002"),
        (format!("typedef float Grid{};", "[2]".repeat(1000)), "S001"),
        (
            format!(
                "typedef float Row{};typedef Row Grid{};",
                "[2]".repeat(32),
                "[2]".repeat(32)
            ),
            "T002",
        ),
    ] {
        fs::write(&input, source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(&input)
            .arg("--check")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains(code));
    }
    fs::remove_file(&input).unwrap();
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_reports_excessive_and_recursive_constructor_defaults() {
    let root = std::env::temp_dir().join(format!("slang-rs-init-depth-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("initialization.slang");
    let mut nested = String::from("struct S0{float value;};");
    for index in 1..40 {
        nested.push_str(&format!("struct S{index}{{S{} value;}};", index - 1));
    }
    nested.push_str("void run(){S39 value=S39();}");
    let mut branching = String::from("struct S0{float x=1;};");
    for index in 1..18 {
        branching.push_str(&format!(
            "struct S{index}{{S{0} a=S{0}();S{0} b=S{0}();}};",
            index - 1
        ));
    }
    branching.push_str("void run(){S17 value={};}");
    let mut arguments = String::from("float f0(){return 1;}");
    for index in 1..18 {
        arguments.push_str(&format!(
            "float f{index}(float x=f{0}()+f{0}()){{return x;}}",
            index - 1
        ));
    }
    arguments.push_str("float run(){return f17();}");
    for (source, code) in [
        (nested, "T020"),
        (branching, "I002"),
        (arguments, "I002"),
        (
            "float read(float x=read()){return x;}float run(){return read();}".into(),
            "I002",
        ),
        (
            "struct S{float values[65537];};void run(){S value=S();}".into(),
            "T020",
        ),
        ("void run(){float values[512][512]={1};}".into(), "T020"),
        ("void run(){float values[65537]={};}".into(), "T020"),
        (
            "struct S{float x=read(S());};float read(S s){return s.x;}void run(){S value=S();}"
                .into(),
            "I002",
        ),
    ] {
        fs::write(&input, source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(&input)
            .arg("--check")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains(code));
    }
    fs::remove_file(&input).unwrap();
    fs::remove_dir(&root).unwrap();
}

#[test]
fn cli_checks_shared_and_deep_type_graphs_without_stack_recursion() {
    let root = std::env::temp_dir().join(format!("slang-rs-type-graph-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("types.slang");
    for (levels, accepted) in [(128, true), (129, false)] {
        let mut source = String::new();
        for index in 0..levels {
            let fields = if index + 1 == levels {
                "float value;".into()
            } else {
                format!("S{0:03} a;S{0:03} b;", index + 1)
            };
            source.push_str(&format!("struct S{index:03}{{{fields}}};"));
        }
        fs::write(&input, source).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_slangc-rs"))
            .arg(&input)
            .arg("--check")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(if accepted { 0 } else { 1 }),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        if !accepted {
            assert!(String::from_utf8_lossy(&result.stderr).contains("T002"));
        }
    }
    fs::remove_file(&input).unwrap();
    fs::remove_dir(&root).unwrap();
}
