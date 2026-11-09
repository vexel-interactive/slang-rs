use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> Compilation {
    compile_source(
        "generic-functions.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn explicit_functions_cache_canonical_arguments_and_check_unused_bodies() {
    let compiled = compile(
        "T copy<T>(T value){return (T)value;}typealias Integer=int;int run(){return copy<int>(2)+copy<Integer>(3)+int(copy<float>(4));}",
    );
    assert_eq!(compiled.ir.functions.len(), 3);
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.return_type == Type::Scalar(Scalar::Float))
    );
    let unused = compile("T copy<T>(T value){return T(value);}int run(){return 1;}");
    assert_eq!(unused.ir.functions.len(), 1);
    assert!(!unused.artifact.unwrap().contains("symbolic"));
}

#[test]
fn dependent_type_defaults_value_defaults_and_output_locations_work() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/generic-function-default-calls.slang"
    ));
    assert!(!compiled.artifact.unwrap().contains("symbolic"));
    compile(
        "T zero<T=int>(){return T();}T pick<T,U=T>(T first,U second=U()){return first;}void assign<T>(out T result,T input){result=(T)input;}void exchange<T>(inout T first,inout T second){T old=first;first=second;second=old;}int run(){int a=2,b=3;assign<int>(a,4);exchange<int>(a,b);return zero<>()+pick<int>(a)+pick<int,float>(b,5);}",
    );
}

#[test]
fn nested_template_calls_support_aggregates_arrays_shapes_and_resources() {
    compile(
        "T copy<T>(T value){return (T)value;}struct Item{int value;};ConstantBuffer<Item> input;int run(){return copy<ConstantBuffer<Item>>(input).value;}",
    );
    compile(
        "T copy<T>(T value){return T(value);}struct Item{int value;};ConstantBuffer<Item> input;int run(){return copy<ConstantBuffer<Item>>(input).value;}",
    );
    compile(
        "T copy<T>(T value){return (T)value;}T relay<T>(T value){return copy<T>(value);}struct Box<T>{T value;T read(){return relay<T>(value);}};struct Item{int value;};StructuredBuffer<int> input;int run(){Box<int[2]> array={{2,3}};Item item={4};return array.read()[1]+relay<Item>(item).value+relay<StructuredBuffer<int>>(input)[0];}",
    );
    compile(
        "vector<T,2> copy<T>(vector<T,2> value){return (vector<T,2>)value;}int run(){return copy<int>(int2(2,3)).y;}",
    );
}

#[test]
fn prototypes_overloads_and_ordinary_functions_preserve_selection() {
    compile(
        "T pick<T>(T a,T b);U pick<U>(U a,U b){return b;}T pick<T>(T a){return a;}int pick(int a){return a+1;}int run(){return pick<int>(2,3)+pick<float>(4)+pick(5);}",
    );
    compile(
        "T relay<T>(T value,int remaining){if(remaining==0)return value;return relay<T>(value,remaining-1);}int run(){return relay<int>(2,3);}",
    );
}

#[test]
fn imported_templates_preserve_declaration_visibility_and_caller_arguments() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;public T copy<T>(T input){return input;}public T relay<T>(T input){return copy<T>(input);}".into())].into() };
    let compiled = compile_source("main.slang", "import lib;struct Private{int value;};int run(){Private input={3};return relay<Private>(input).value;}", &CompileOptions::default(), &includes).unwrap();
    assert!(!compiled.artifact.unwrap().contains("symbolic"));
}

#[test]
fn invalid_templates_and_specializations_report_source_diagnostics() {
    for (source, code) in [
        ("T copy<T>(T value){return missing;}", "T011"),
        ("T copy<T>(T value){return value+value;}", "T012"),
        ("T copy<T>(T value){return (T)3;}", "T008"),
        (
            "T copy<T>(T value){return value;}int run(){return copy<int,float>(2);}",
            "T002",
        ),
        (
            "T copy<T>(T value){return value;}int run(){return copy<>(2);}",
            "T002",
        ),
        (
            "T copy<T>(T value){return value;}int run(){return copy<Missing>(2);}",
            "T002",
        ),
        (
            "T copy<T>(T value){return value;}int run(){return copy<int>();}",
            "T014",
        ),
        (
            "void assign<T>(out T result,T value){result=value;}void run(){assign<int>(1,2);}",
            "T015",
        ),
    ] {
        let error = compile_source(
            "generic-functions.slang",
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
