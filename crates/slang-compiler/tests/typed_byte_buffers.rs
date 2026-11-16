use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str, options: CompileOptions) -> Compilation {
    compile_source("typed-byte.slang", source, &options, &MemoryIncludes::default()).unwrap()
}

#[test]
fn typed_and_inferred_stores_preserve_numeric_types() {
    let source = "RWByteAddressBuffer data;void run(){float x=data.Load<float>(0);data.Store(4,1.5);data.Store(8,int(-2));data.Store(12,true);data.Store<float2>(16,float2(x,2));data.Store2(24,uint2(3,4));}";
    let compiled = compile(source, CompileOptions::default());
    let artifact = compiled.artifact.unwrap();
    assert!(artifact.contains(".Load<float>(_address)"), "{artifact}");
    assert!(artifact.contains(".Store<float>(_address, _value)"), "{artifact}");
    assert!(artifact.contains(".Store<int>(_address, _value)"), "{artifact}");
    assert!(artifact.contains(".Store(_address, ((uint)_value))"), "{artifact}");
    assert!(artifact.contains(".Store2("), "{artifact}");
    assert!(!artifact.contains("@slang.byte."));
    for index in 0..compiled.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
            graph.verify().unwrap();
        }
    }
}

#[test]
fn natural_layout_preserves_tail_reuse_array_stride_and_matrix_order() {
    let source = "struct Inner{uint64_t wide;float tail;};struct Values{Inner inner;float reuse;row_major float2x3 rows;column_major float2x3 columns;half narrow[2];};ByteAddressBuffer input;RWByteAddressBuffer output;void run(){Values x=input.Load<Values>(0);output.Store(0,x);}";
    let artifact = compile(source, CompileOptions::default()).artifact.unwrap();
    for expected in [
        ".Load<uint64_t>(_address)",
        ".Load<float>((_address + 8u))",
        ".Load<float>((_address + 12u))",
        ".Load<float>((_address + 16u))",
        ".Load<float>(((_address + 16u) + 20u))",
        ".Load<float>((_address + 40u))",
        ".Load<float>(((_address + 40u) + 20u))",
        "(_address + 64u) + (_index_",
        "* 2u)",
        "f16tof32",
        "InterlockedCompareExchange",
    ] {
        assert!(artifact.contains(expected), "missing {expected}: {artifact}");
    }
    assert!(artifact.contains("for ("), "{artifact}");
}

#[test]
fn native_narrow_storage_uses_two_byte_typed_accesses() {
    let source = "ByteAddressBuffer input;RWByteAddressBuffer output;void run(){half2 x=input.Load<half2>(0);output.Store(0,x);int16_t y=input.Load<int16_t>(4);output.Store(4,y);}";
    let compiled = compile(source, CompileOptions::default());
    assert!(compiled.ir.native_16bit_types);
    let artifact = compiled.artifact.unwrap();
    assert!(artifact.contains(".Load<half>((_address + 2u))"), "{artifact}");
    assert!(artifact.contains(".Load<int16_t>(_address)"), "{artifact}");
    assert!(!artifact.contains("InterlockedCompareExchange"));
}

#[test]
fn fixed_array_results_and_generic_member_names_keep_their_routes() {
    let source = "typedef float Pair[2];struct Reader{T Load<T>(T x){return x;}T Store<T>(T x){return x;}};ByteAddressBuffer input;RWByteAddressBuffer output;T read<T>(uint offset){return input.Load<T>(offset);}void run(){Pair x=read<Pair>(0);output.Store<Pair>(0,x);Reader r;float y=r.Load<float>(2);output.Store(8,r.Store<float>(y));}";
    let artifact = compile(source, CompileOptions::default()).artifact.unwrap();
    assert!(artifact.contains("_slang_array_result_"), "{artifact}");
    assert!(artifact.contains(".Load<float>"), "{artifact}");
    assert!(!artifact.contains("@slang.byte."));
}

#[test]
fn aligned_methods_accept_default_zero_and_valid_constant_promises() {
    let source = "ByteAddressBuffer input;RWByteAddressBuffer output;static const uint alignment=8;void run(){float x=input.LoadAligned<float>(0);x=input.LoadAligned<float>(8,alignment);x=input.LoadAligned<float>(1,0);x=input.Load<float>(1);output.Store<float>(0,x,4);output.StoreAligned<float>(0,x);}";
    compile(source, CompileOptions::default());
}

#[test]
fn invalid_typed_accesses_report_semantic_errors() {
    for source in [
        "ByteAddressBuffer input;void run(){input.Store<float>(0,1);}",
        "ByteAddressBuffer input;void run(){input.Load<float,int>(0);}",
        "ByteAddressBuffer input;void run(){input.Load<float>();}",
        "ByteAddressBuffer input;void run(){input.LoadAligned<float>(0,3);}",
        "ByteAddressBuffer input;void run(){input.LoadAligned<double>(0,4);}",
        "ByteAddressBuffer input;void run(){input.LoadAligned<float>(2,4);}",
        "ByteAddressBuffer input;void run(){input.LoadAligned<float>(1);}",
        "RWByteAddressBuffer output;void run(){output.StoreAligned<half>(1,half(1));}",
        "ByteAddressBuffer input;void run(uint alignment){input.LoadAligned<float>(0,alignment);}",
        "ByteAddressBuffer input;void run(){input.Load<Texture2D<float>>(0);}",
        "struct Bad{SamplerState sampler;};ByteAddressBuffer input;void run(){input.Load<Bad>(0);}",
    ] {
        let error = compile_source(
            "invalid-byte.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T014"),
            "{source}: {error}"
        );
    }
}

#[test]
fn every_numeric_scalar_and_shape_has_typed_raw_storage() {
    for scalar in [
        "bool",
        "int16_t",
        "uint16_t",
        "int",
        "uint",
        "int64_t",
        "uint64_t",
        "intptr_t",
        "uintptr_t",
        "half",
        "float",
        "double",
    ] {
        for suffix in ["", "1", "4", "2x3"] {
            let ty = if matches!(scalar, "intptr_t" | "uintptr_t") {
                match suffix {
                    "" => scalar.to_owned(),
                    "2x3" => format!("matrix<{scalar},2,3>"),
                    width => format!("vector<{scalar},{width}>"),
                }
            } else {
                format!("{scalar}{suffix}")
            };
            let source = format!(
                "ByteAddressBuffer input;RWByteAddressBuffer output;void run(){{{ty} value=input.Load<{ty}>(0);output.Store(0,value);}}"
            );
            let compiled = compile(&source, CompileOptions::default());
            assert!(!compiled.artifact.unwrap().contains("@slang.byte."), "{source}");
        }
    }
}

#[test]
fn resource_address_value_and_void_access_arguments_are_evaluated_once() {
    let source = "RWByteAddressBuffer output;static uint calls;RWByteAddressBuffer receiver(){calls+=1;return output;}uint address(){calls+=2;return 0;}float value(){calls+=4;return 1;}void run(){receiver().Store<float>(address(),value());receiver().Load<void>(address());}";
    let artifact = compile(source, CompileOptions::default()).artifact.unwrap();
    let run = artifact.split("void run()").last().unwrap().split("void _slang_").next().unwrap();
    assert_eq!(run.matches("receiver()").count(), 2, "{run}");
    assert_eq!(run.matches("address()").count(), 2, "{run}");
    assert_eq!(run.matches("value()").count(), 1, "{run}");
}

#[test]
fn legacy_half_stores_use_a_retrying_word_exchange() {
    let source = include_str!("../../../tests/conformance/positive/typed-byte-half-legacy.slang");
    let compiled = compile(source, CompileOptions::default());
    assert!(!compiled.ir.native_16bit_types);
    let artifact = compiled.artifact.unwrap();
    assert!(artifact.contains("f16tof32"), "{artifact}");
    assert!(artifact.contains("f32tof16"), "{artifact}");
    assert!(artifact.contains("InterlockedCompareExchange"), "{artifact}");
    assert!(artifact.contains("while ((_observed_0 != _expected_0))"), "{artifact}");
    assert!(!artifact.contains(".Store<half>"), "{artifact}");
}

#[test]
fn large_arrays_use_loops_and_exponential_structs_fail_with_a_bound() {
    let source = "typedef float Huge[100000000];RWByteAddressBuffer output;void run(Huge value){output.Store(0,value);}";
    let artifact = compile(source, CompileOptions::default()).artifact.unwrap();
    assert!(artifact.contains("< 100000000u"), "{artifact}");
    assert!(artifact.len() < 10000);

    let mut source = "struct Level0{float value;};".to_owned();
    for level in 1..=17 {
        let previous = level - 1;
        source.push_str(&format!(
            "struct Level{level}{{Level{previous} left;Level{previous} right;}};"
        ));
    }
    source.push_str("ByteAddressBuffer input;void run(){input.Load<Level17>(0);}");
    let error = compile_source(
        "bounded-byte.slang",
        &source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|diagnostic| diagnostic.code == "I002"
            && diagnostic.message.contains("expansion limit")),
        "{error}"
    );
}

#[test]
fn inferred_matrix_stores_keep_the_source_alias_layout() {
    for (major, stride) in [("row_major", 4), ("column_major", 8)] {
        let other_major = if major == "row_major" {
            "column_major"
        } else {
            "row_major"
        };
        for expression in ["value", "value+value", "value+other", "value+1", "1+value"] {
            let source = format!(
                "typedef {major} float2x3 Matrix;typedef {other_major} float2x3 Other;RWByteAddressBuffer output;void run(){{Matrix value=Matrix(1,2,3,4,5,6);Other other=Other(6,5,4,3,2,1);output.Store(0,{expression});}}"
            );
            let artifact = compile(&source, CompileOptions::default()).artifact.unwrap();
            let expected = format!(".Store<float>((_address + {stride}u), ((_value)[0u])[1u])");
            assert!(artifact.contains(&expected), "{artifact}");
        }
    }
}
