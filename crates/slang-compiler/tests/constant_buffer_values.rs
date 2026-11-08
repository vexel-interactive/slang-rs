use slang_compiler::{CompileOptions, MatrixMajor, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "values.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn all_numeric_element_shapes_load_as_values_and_keep_source_reflection() {
    for scalar in ["bool", "int", "uint", "half", "float", "double"] {
        let mut names = vec![scalar.to_owned()];
        names.extend((1..=4).map(|n| format!("{scalar}{n}")));
        for rows in 1..=4 {
            names.extend((1..=4).map(|columns| format!("{scalar}{rows}x{columns}")));
        }
        for name in names {
            let compiled = compile(&format!(
                "typedef {name} Value;ConstantBuffer<Value> input;Value read(){{return input;}}Value copy(Value v){{return v;}}Value f(){{Value local=input;return copy(input);}}"
            ));
            let input = &compiled.reflection.globals[0];
            assert_eq!(input.type_name, format!("ConstantBuffer<{name}>"));
            assert_eq!(input.uniform_layout.as_ref().unwrap().type_name, name);
            assert_eq!(input.resource_access.as_deref(), Some("read"));
            let read = compiled
                .ir
                .functions
                .iter()
                .find(|f| f.name == "read")
                .unwrap();
            assert_eq!(read.return_type, Type::builtin(&name).unwrap());
            assert!(
                !compiled
                    .artifact
                    .unwrap()
                    .contains(&format!("ConstantBuffer<{name}>"))
            );
        }
    }
}

#[test]
fn overloaded_calls_distinguish_resource_handles_from_loaded_numeric_values() {
    let compiled = compile(
        "ConstantBuffer<float> input;float choose(float v){return v;}double choose(double v){return v;}uint select(ConstantBuffer<float> v){return 1u;}float select(float v){return v;}float f(){return choose(input);}uint g(){return select(input);}",
    );
    for (name, expected) in [
        ("f", Type::Scalar(Scalar::Float)),
        ("g", Type::Scalar(Scalar::Uint)),
    ] {
        let function = compiled
            .ir
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        let slang_ir::StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
            panic!()
        };
        let slang_ir::StmtKind::Return(Some(value)) = &statements[0].kind else {
            panic!()
        };
        assert_eq!(value.ty, expected);
        let slang_ir::ExprKind::Call { arguments, .. } = &value.kind else {
            panic!()
        };
        if name == "g" {
            assert!(matches!(arguments[0].ty, Type::ConstantBuffer { .. }));
        } else {
            assert_eq!(arguments[0].ty, Type::Scalar(Scalar::Float));
        }
    }
    let compiled = compile(
        "struct S{float x;};ConstantBuffer<S> input;float choose(S value){return value.x;}uint choose(ConstantBuffer<S> value){return 1u;}float f(){return choose(S(input));}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("choose(((S)input))"));
}

#[test]
fn value_contexts_include_initializers_intrinsics_conditions_indices_and_switches() {
    compile(
        "ConstantBuffer<float> scalar;ConstantBuffer<float2> vector;ConstantBuffer<int> index;ConstantBuffer<bool> condition;typedef float3 Values[2];ConstantBuffer<Values> array;float f(){float3 flat={scalar,vector};float3 constructed=float3(vector,scalar);Values copy=Values(array);float x=sin(scalar)+dot(vector,vector)+array[index].z;int selected=index;switch(selected){case 0:x++;break;default:break;}if(condition)x+=flat.x;return condition?x:scalar;}",
    );
    compile(
        "ConstantBuffer<float> input;float read(ConstantBuffer<float> value){return value;}float f(){return read((0,input));}",
    );
}

#[test]
fn matrix_and_array_qualifiers_survive_resource_elements_and_compilation_defaults() {
    for major in [MatrixMajor::Row, MatrixMajor::Column] {
        let compiled=compile_source("layout.slang","typedef row_major float2x3 Row;typedef Row Values[2];ConstantBuffer<Row> matrix;ConstantBuffer<Values> arrays[2]:register(b3,space2);float f(){return matrix[1][2]+arrays[1][0][1][2];}",&CompileOptions {matrix_major:major,..CompileOptions::default()},&MemoryIncludes::default()).unwrap();
        assert_eq!(
            compiled.reflection.globals[0]
                .uniform_layout
                .as_ref()
                .unwrap()
                .type_name,
            "row_major float2x3"
        );
        assert_eq!(
            compiled.reflection.uniform_types["row_major float2x3"].size,
            28
        );
        assert_eq!(compiled.reflection.globals[1].binding.unwrap().count, 2);
        assert_eq!(
            compiled.reflection.uniform_types["row_major float2x3[2]"].size,
            60
        );
        assert!(
            compiled
                .artifact
                .unwrap()
                .contains("row_major float2x3 _value[2]")
        );
    }
}

#[test]
fn direct_large_array_reads_are_compact_and_materialized_copies_are_bounded() {
    let source = "typedef float3 Values[100000];ConstantBuffer<Values> input;float f(){return input[99999].z;}";
    let compiled = compile(source);
    assert_eq!(
        compiled.reflection.uniform_types["float3[100000]"].size,
        1_599_996
    );
    assert!(compiled.artifact.unwrap().len() < 4096);
    let source =
        "typedef float3 Values[100000];ConstantBuffer<Values> input;void f(){Values values=input;}";
    let error = compile_source(
        "huge.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "I004"));
}

#[test]
fn direct_packed_and_imported_struct_elements_convert_to_their_value_types() {
    compile(
        "struct S{float x;};ConstantBuffer<S> input;float read(S v){return v.x;}float f(){S value=input;return read(input)+value.x;}",
    );
    compile(
        "struct S{float x:packoffset(c1);};ConstantBuffer<S> input;float read(S v){return v.x;}float f(){S value=input;return read(input)+value.x;}",
    );
    let compiled=compile_source("main.slang","import library;float f(){S value=input;return read(input)+value.x;}",&CompileOptions::default(),&MemoryIncludes {files:std::collections::BTreeMap::from([("library.slang".into(),"module library;public struct S{public float x;};public ConstantBuffer<S> input;public float read(S value){return value.x;}".into())])}).unwrap();
    assert_eq!(
        compiled.reflection.globals[0].module.as_deref(),
        Some("library")
    );
}

#[test]
fn contents_remain_read_only_in_numeric_and_aggregate_output_contexts() {
    for source in [
        "ConstantBuffer<float> input;void f(){input++;}",
        "void f(ConstantBuffer<float> input){input++;}",
        "void set(out float value){value=1;}void f(ConstantBuffer<float> input){set(input);}",
        "void f(ConstantBuffer<float3> input){input.x=1;}",
        "typedef float3 Values[2];void f(ConstantBuffer<Values> input){input[1].z=1;}",
        "StructuredBuffer<float> input;void f(ConstantBuffer<uint> size){uint stride;input.GetDimensions(size,stride);}",
        "void f(ConstantBuffer<float> input){float fraction;modf(1.0,input);}",
    ] {
        let error = compile_source(
            "readonly.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T015"),
            "{source}: {error}"
        );
    }
    for source in [
        "ConstantBuffer<Texture2D<float>> input;",
        "typealias Values=Texture2D<float>[2];ConstantBuffer<Values> input;",
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == "T002"));
    }
}

#[test]
fn void_elements_do_not_allocate_bindings_and_loads_keep_index_evaluation() {
    let compiled = compile(
        "ConstantBuffer<void> input[2]:register(b0);struct S{float x;};ConstantBuffer<S> data;static uint count=0;uint next(){return count++;}void read(){return input[next()];}void take(ConstantBuffer<void> value){}void f(){read();take(input[next()]);}",
    );
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "ConstantBuffer<void>[2]"
    );
    assert!(compiled.reflection.globals[0].binding.is_none());
    assert!(compiled.reflection.globals[0].uniform_layout.is_none());
    assert_eq!(
        compiled.reflection.globals[1]
            .binding
            .unwrap()
            .register
            .index,
        0
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("void _value"));
    assert_eq!(code.matches("[next()]").count(), 2);
}

#[test]
fn aggregate_handle_returns_and_defaults_load_without_repeating_side_effects() {
    let compiled = compile(
        "struct S{float3 values[2];row_major float2x3 matrix;};typedef S Values[2];ConstantBuffer<Values> input[2];static uint count=0;uint next(){return count++;}ConstantBuffer<Values> handle(uint i){return input[i];}float read(Values values=input[0]){return values[1].matrix[1][2];}float f(){Values copy=handle(next());return read(handle(next()))+read()+copy[0].values[1].z;}",
    );
    assert_eq!(compiled.reflection.uniform_types["S[2]"].size, 124);
    assert_eq!(
        compiled.artifact.unwrap().matches("handle(next())").count(),
        2
    );
}

#[test]
fn switch_selectors_require_an_explicit_integer_value() {
    let error = compile_source(
        "switch.slang",
        "void f(ConstantBuffer<int> input){switch(input){case 0:break;}}",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T009"));
}
