use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> Compilation {
    compile_source(
        "generic-inference.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn semantic_arguments_infer_types_before_defaults_and_reuse_instances() {
    let compiled = compile(
        "T copy<T=int>(T value){return value;}T relay<T>(T value){return copy(value);}int run(){int a=2;float b=3;return copy(a)+copy<>(2)+int(relay(b));}",
    );
    assert_eq!(compiled.ir.functions.len(), 4);
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.name != "run" && f.return_type == Type::Scalar(Scalar::Float))
    );
    assert!(!compiled.artifact.unwrap().contains("symbolic"));
    compile(
        "U pick<T,U=T>(T first,U second=U()){return second;}float run(){return pick<int>(2,3.0)+pick(2.0);}",
    );
}

#[test]
fn nested_calls_and_symbolic_shapes_infer_structural_parameters() {
    compile(
        "T copy<T>(T value){return (T)value;}vector<T,2> relay<T>(vector<T,2> value){return copy(value);}int run(){return relay(copy(int2(2,3))).y;}",
    );
    compile(
        "matrix<T,2,2> relay<T>(matrix<T,2,2> value){return value;}int run(){return relay(int2x2(2,3,4,5))[1][1];}",
    );
    compile(
        "T copy<T>(T value){return value;}T first<T>(T values[2]){return values[0];}int run(){int a[2]={2,3};return first(copy(a));}",
    );
    compile(
        "vector<T,2> relay<T>(vector<T,2> value){return value;}float2 run(){return relay(2.0);}",
    );
    compile(
        "T pick<T>(T first,T second){return second;}vector<intptr_t,2> run(){vector<intptr_t,2> first=vector<intptr_t,2>(2);return pick(first,vector<int64_t,2>(3));}",
    );
}

#[test]
fn aggregates_resources_and_owner_arguments_are_preserved() {
    compile(
        "T copy<T>(T value){return T(value);}struct Item{int value;};ConstantBuffer<Item> input;int run(){return copy(input).value;}",
    );
    compile(
        "T copy<T>(T value){return value;}struct Box<T>{T value;T read(){return copy(value);}};T get<T>(Box<T> box){return box.value;}int run(){Box<int> box={2};return get(box)+box.read();}",
    );
    compile(
        "StructuredBuffer<T> copy<T>(StructuredBuffer<T> value){return value;}StructuredBuffer<int> input;int run(){return copy(input)[0];}",
    );
    compile(
        "T at<T>(T values[2]){return values[0];}ConstantBuffer<int[2]> input;int run(){return at(input);}",
    );
}

#[test]
fn nested_owner_and_defaulted_user_type_arguments_are_inferred() {
    compile(
        "struct Outer<T>{typealias Value=T;};T copy<T>(Outer<T>.Value value){return value;}int run(){return copy(2);}",
    );
    compile(
        "struct A<T>{struct Inner{T value;};};struct B<T>{struct Inner{T value;};};int choose<T>(A<T>.Inner value){return 1;}int choose<T>(B<T>.Inner value){return 2;}int run(){B<int>.Inner value={3};return choose(value);}",
    );
    compile(
        "struct Pair<T,U=int>{T first;U second;};T get<T>(Pair<T> pair){return pair.first;}int run(){Pair<int> pair={2,3};return get(pair);}",
    );
    compile(
        "struct Outer<T>{struct Inner<U>{T first;U second;};};U get<T,U>(Outer<T>.Inner<U> value){return value.second;}int run(){Outer<float>.Inner<int> value={2.0,3};return get(value);}",
    );
}

#[test]
fn ordinary_and_generic_overloads_compete_and_outputs_stay_writable() {
    let compiled = compile(
        "T pick<T>(T value){return value;}int pick(int value){return value+1;}int run(){return pick(2)+int(pick(3.0));}",
    );
    assert_eq!(compiled.ir.functions.len(), 4);
    compile(
        "void assign<T>(out T result,T value){result=value;}void exchange<T>(inout T first,inout T second){T old=first;first=second;second=old;}int run(){int a=2,b=3;assign(a,4);exchange(a,b);return a;}",
    );
    compile(
        "T copy<T>(T value){return value;}vector<T,2> copy<T>(vector<T,2> value){return value;}StructuredBuffer<int> input;int run(){return copy(input)[0];}",
    );
}

#[test]
fn inference_failures_and_unused_body_errors_are_not_hidden() {
    for (source, code) in [
        (
            "T create<T>(){return T();}int run(){return create();}",
            "T002",
        ),
        (
            "T copy<T>(T value){return value;}int run(){return copy({2});}",
            "T002",
        ),
        (
            "T pick<T>(T a,T b){return a;}int run(){int a[2]={1,2};float b[2]={3,4};return int(pick(a,b)[0]);}",
            "T002",
        ),
        (
            "T copy<T>(T value){return value;}vector<T,2> copy<T>(vector<T,2> value){return value;}int run(){return copy(int2(2,3)).x;}",
            "T014",
        ),
        (
            "T copy<T>(T value){return value;}T relay<T>(T value){return copy(value)+value;}",
            "T012",
        ),
        (
            "T copy<T>(T value){return value;}T relay<T>(T value){return copy(missing);}",
            "T011",
        ),
        (
            "void assign<T>(out T result,T value){result=value;}void run(){assign(1,2);}",
            "T015",
        ),
        (
            "T copy<T>(T value){return value;}int run(){int copy=2;return copy(3);}",
            "T014",
        ),
    ] {
        let error = compile_source(
            "generic-inference.slang",
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
    for call in [
        "pick({1,2},int2(3,4))",
        "pick(int2(3,4),{1,2})",
        "defaulted({2})",
    ] {
        let source = format!(
            "T pick<T>(T first,T second){{return second;}}T defaulted<T=int>(T value){{return value;}}int run(){{return {call}.x;}}"
        );
        let error = compile_source(
            "generic-inference.slang",
            &source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T002"),
            "{error}"
        );
    }
}
