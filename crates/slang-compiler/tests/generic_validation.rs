use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "symbolic.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn unused_templates_receive_ordinary_name_type_and_control_flow_checks() {
    for (source, code) in [
        ("struct Box<T>{int read(){return missing;}};", "T011"),
        ("struct Box<T>{int value=\"invalid\";};", "T001"),
        (
            "int read(int x){return x;}struct Box<T>{int get(){return read();}};",
            "T014",
        ),
        ("struct Box<T>{int get(){break;return 0;}};", "T007"),
        (
            "struct Box<T>{int get(){int value;int value;return 0;}};",
            "T003",
        ),
        ("struct Box<T>{int get(){if(true)return 0;}};", "T005"),
        ("struct Box<T>{Box get(){return {};}};", "T002"),
    ] {
        let error = compile_source(
            "symbolic.slang",
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

#[test]
fn unconstrained_parameters_cannot_use_numeric_members_operators_or_conversions() {
    for (source, code) in [
        (
            "struct Box<T>{T value;T add(T other){return value+other;}};",
            "T012",
        ),
        (
            "struct Box<T>{T value;T negative(){return -value;}};",
            "T012",
        ),
        (
            "struct Box<T>{T value;int get(){return value.missing;}};",
            "T013",
        ),
        (
            "struct Box<T>{T value;int get(){if(value)return 1;return 0;}};",
            "T009",
        ),
        ("struct Box<T>{T value;T get(){return T(3);}};", "T014"),
        ("struct Box<T>{T value;T get(){return 3;}};", "T008"),
        (
            "struct Box<T>{vector<T,2> value;vector<T,2> add(vector<T,2> other){return value+other;}};",
            "T012",
        ),
    ] {
        let error = compile_source(
            "symbolic.slang",
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

#[test]
fn opaque_copy_default_and_shape_operations_leave_no_symbolic_types_in_output() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-symbolic-values.slang"
    ));
    let ir = serde_json::to_string(&compiled.ir).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(!ir.contains("symbolic_shape") && !ir.contains("parameter_"));
    assert!(!code.contains("symbolic_shape") && !code.contains("parameter_"));
}

#[test]
fn valid_unused_templates_are_checked_without_becoming_emitted_instances() {
    let compiled = compile(
        "struct Holder<T>{T value;T copy(){return T(value);}T create(){return T();}};struct Pair<T,U=T>{T first;U second;};void run(){}",
    );
    assert!(compiled.ir.structures.is_empty());
    assert_eq!(compiled.ir.functions.len(), 1);
    compile(
        "struct MatrixHolder<T>{row_major matrix<T,2,3> value;vector<T,3> row(){return value[0];}};",
    );
}

#[test]
fn imported_unused_templates_cannot_capture_root_names_or_private_types() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public struct Box<T>{public int read(){return secret;}};".into(),
        )]
        .into(),
    };
    let error = compile_source(
        "main.slang",
        "import lib;static int secret=3;void run(){}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T011"),
        "{error}"
    );
    assert!(error.to_string().contains("lib.slang:1:"));
}
