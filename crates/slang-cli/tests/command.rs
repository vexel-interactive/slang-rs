use std::{fs, process::Command};

#[test]
fn cli_rejects_output_aliases_of_sources() {
    let root = std::env::temp_dir().join(format!("slang-rs-cli-alias-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let input = root.join("valid.slang");
    let included = root.join("included.slang");
    let source = "#include \"included.slang\"\nvoid f(){}";
    fs::write(&input, source).unwrap();
    fs::write(&included, "const int value=1;").unwrap();
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
    assert_eq!(fs::read_to_string(&included).unwrap(), "const int value=1;");
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
