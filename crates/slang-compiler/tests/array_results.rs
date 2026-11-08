use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "results.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn arrays_return_from_prototypes_branches_and_direct_types() {
    let compiled = compile(
        "typedef float V[Count];static const int Count=3;V choose(bool flag);V choose(bool flag){if(flag)return {1,2,3};return V(4,5,6);}float[3] copy(V x){return x;}float f(){V x=copy(choose(true));return x[2]+choose(false)[1];}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _value[3]"));
    assert!(code.contains("_slang_array_result_0 choose(bool flag)"));
    assert!(code.contains("(choose(false))._value"));
}

#[test]
fn defaults_globals_overloads_and_nested_arrays_keep_source_shapes() {
    let compiled = compile(
        "typedef float V[2];typedef float M[2][3];V make(){return V(1,2);}static V global=make();V choose(V x=make()){return x;}M choose(M x){return x;}float f(){V v=choose();M m=choose(float[3][2](1,2,3,4,5,6));return global[1]+v[1]+m[1][2];}",
    );
    assert_eq!(compiled.reflection.globals[0].type_name, "float[2]");
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _value[2][3]"));
    assert!(code.contains("float global[2]"));
}

#[test]
fn imported_returns_and_resource_value_loads_remain_checked() {
    compile_source("main.slang","import library;ConstantBuffer<Values> input;float f(){Values a=copy(input);return a[2]+load(input)[1];}",&CompileOptions::default(),&MemoryIncludes {files:[("library.slang".into(),"module library;public typedef float Values[3];public Values copy(Values x){return x;}public Values load(ConstantBuffer<Values> x){return x;}".into())].into()}).unwrap();
}

#[test]
fn recursion_and_side_effecting_returns_evaluate_the_call_once() {
    let compiled = compile(
        "typedef float V[2];V make(inout int i){i++;return V(i,i+1);}V recur(int i){if(i==0)return V();return recur(i-1);}float f(){int i=0;V a=make(i);return a[1]+recur(2)[0];}",
    );
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("make(i)").count(), 1);
    assert_eq!(code.matches("recur((i - 1))").count(), 1);
}

#[test]
fn matrix_return_qualifiers_move_to_private_storage() {
    let compiled = compile(
        "typedef row_major float2x3 M[2];M make(){return M();}float f(){M m=make();return m[1][1][2];}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("row_major float2x3 _value[2]"));
    assert!(!code.contains("row_major _slang__slang_array_result"));
}

#[test]
fn resource_array_overloads_keep_distinct_wrappers_and_initialized_handles() {
    let compiled = compile(
        "typedef Texture2D<float4> T[2];typedef SamplerState S[2];T copy(T x){return x;}S copy(S x){return x;}T textures;S samplers;float4 f(){return copy(textures)[1].SampleLevel(copy(samplers)[0],float2(0),0);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("(copy(textures))._value"));
    assert!(code.contains("(copy(samplers))._value"));
    assert!(!code.contains("((_slang__slang_array_result_0)0)"));
    assert!(!code.contains("((_slang__slang_array_result_1)0)"));
    compile(
        "typedef Texture2D<float4> T[2];T images;T get(){return images;}T copy(T x=get()){return x;}float4 f(){return copy()[1][int2(0)];}",
    );
}

#[test]
fn invalid_array_returns_still_fail_before_lowering() {
    for (source, code) in [
        ("float[2] f(){return float[3]();}", "T008"),
        ("float[2] f(){return {1,2,3};}", "T021"),
        ("float[2] f(bool flag){if(flag)return {1,2};}", "T005"),
        ("float[2] f(){return {1,2};}void g(){f()[0]=1;}", "T015"),
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
