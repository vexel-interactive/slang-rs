use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "arrays.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn direct_and_alias_constructors_preserve_dimension_order() {
    let compiled = compile(
        "typedef float Row[3];static Row values=Row(1,2,3);float run(){Row a=float[3](4);Row b=Row(a);float nested[2][3]=float[3][2](float[3](1,2,3),Row(4,5,6));return b[2]+nested[1][2]+values[2];}",
    );
    assert_eq!(
        compiled.ir.globals[0].ty,
        Type::Array(Box::new(Type::Scalar(Scalar::Float)), 3)
    );
    assert_eq!(compiled.reflection.globals[0].type_name, "float[3]");
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("Row("));
    assert!(code.contains("float _value[3]"));
    assert!(code.contains("_value[2]"));
}

#[test]
fn constructors_flatten_or_respect_nested_argument_boundaries() {
    let compiled = compile(
        "float sum(float a[2][3]){return a[0][2]+a[1][2];}float run(){float a[2][3]=float[3][2](1,2,3,4,5,6);float b[2][3]=float[3][2]({1},{4,5,6});return sum(a)+sum(b)+sum(float[3][2]());}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _value[3]"));
    assert!(code.contains("sum("));
    assert!(!code.contains("float[3]("));
}

#[test]
fn struct_array_defaults_and_copies_do_not_repeat_supplied_values() {
    let compiled = compile(
        "float next(){return 7;}struct P{float x=next();};typedef P Values[2];float run(){Values a=P[2]();Values b=Values(a);return b[1].x;}",
    );
    assert_eq!(compiled.artifact.unwrap().matches("next()").count(), 4);
    let files = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            "module lib;public struct P{public float x=3;};public typedef P Values[2];".into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        "import lib;float run(){Values a=Values({1});return a[1].x;}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
}

#[test]
fn invalid_constructor_types_and_excess_elements_are_diagnostics() {
    for (source, code) in [
        ("void run(){float a[2]=float[2](1,2,3);}", "T021"),
        ("void run(){float a[2][2]=float[2][2]({1,2,3});}", "T021"),
        ("void run(){float a[2]=float[0]();}", "T002"),
        ("void run(){float a[2]=float[-1]();}", "T002"),
        ("void run(){float a[2]=float[2.0]();}", "T002"),
        ("void run(){float a[2]=float[65537]();}", "T020"),
        ("void run(){float a[2]=void[2]();}", "T002"),
        ("void run(){float a[2]=Texture2D[2]();}", "T002"),
        ("void run(){float values[2];values[1]();}", "T001"),
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
