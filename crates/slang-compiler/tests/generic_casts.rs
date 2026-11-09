use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "generic-casts.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn parameter_copy_casts_preserve_scalar_aggregate_array_and_resource_values() {
    let compiled = compile(
        "struct Box<T>{T value;T read(){return (T)value;}};struct Item{int value;};StructuredBuffer<int> input;int run(){Box<int> scalar={2};Box<int[2]> array={{3,4}};Box<Item> item={{5}};Box<StructuredBuffer<int>> resource={input};return scalar.read()+array.read()[1]+item.read().value+resource.read()[0];}",
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("((StructuredBuffer<int>)"), "{code}");
    assert!(!code.contains("symbolic"));
}

#[test]
fn nested_type_parameters_and_numeric_shapes_support_copy_casts() {
    compile(
        "struct Outer<T>{struct Inner<U>{T first;U second;T read(){return (T)first;}U other(){return (U)second;}};};float run(){Outer<int>.Inner<float> value={2,3};return value.read()+value.other();}",
    );
    compile(
        "struct Shapes<T>{vector<T,2> value;vector<T,2> read(){return (vector<T,2>)value;}};int run(){Shapes<int> value={{2,3}};return value.read().y;}",
    );
}

#[test]
fn parameter_casts_do_not_grant_unconstrained_conversions() {
    let error = compile_source(
        "generic-casts.slang",
        "struct Box<T>{T read(){return (T)3;}};",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T008"),
        "{error}"
    );
    compile("struct Box<T>{T value;};int run(int x,int y){Box<int> value={2};return (x)+y;}");
}
