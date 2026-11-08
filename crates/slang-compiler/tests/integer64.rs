use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, Severity, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "wide.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn unsigned_constants_preserve_high_bits_and_typed_operations() {
    let compiled = compile(
        "static const uint64_t Max=0xffffffffffffffffULL;static const uint64_t Quotient=Max/3ULL;static const uint64_t Remainder=Max%7ULL;static const uint64_t Shift=Max>>63;static const bool Compare=Max>0x7fffffffffffffffLL;static const int Count=int(Shift)+int(Compare)+int(Remainder);float data[Count];",
    );
    assert_eq!(
        compiled.reflection.globals.last().unwrap().type_name,
        "float[3]"
    );
    let code = compiled.artifact.unwrap();
    for text in [
        "Max = 18446744073709551615ull;",
        "Quotient = 6148914691236517205ull;",
        "Remainder = 1ull;",
        "Shift = 1ull;",
        "Compare = true;",
    ] {
        assert!(code.contains(text), "{text}: {code}");
    }
}

#[test]
fn signed_constants_conversions_and_forward_shapes_keep_64_bits() {
    let compiled = compile(
        "static const int64_t Signed=int64_t(uint(0xffffffff));static const int64_t Min=-9223372036854775808;static const int64_t Shift=Min>>62;float data[Count];static const int64_t Count=(1LL<<40)>>39;static const uint64_t Wrapped=uint64_t(-1);static const uint64_t Selected=true?Wrapped:0ULL;int64_t convert(){return int64_t(1099511627776.0L);}",
    );
    assert_eq!(compiled.reflection.globals[3].type_name, "float[2]");
    let code = compiled.artifact.unwrap();
    for text in [
        "Signed = 4294967295ll;",
        "Min = (-9223372036854775807ll - 1ll);",
        "Shift = -2ll;",
        "Wrapped = 18446744073709551615ull;",
        "Selected = 18446744073709551615ull;",
    ] {
        assert!(code.contains(text), "{text}: {code}");
    }
}

#[test]
fn literal_minima_fold_only_across_direct_prefixes() {
    let compiled = compile(
        "int f(){return -2147483648;}int64_t g(){return -9223372036854775808;}int64_t h(){return -(2147483648);}uint64_t j(){return -(9223372036854775808);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return (-2147483647 - 1);"), "{code}");
    assert!(
        code.contains("return (-9223372036854775807ll - 1ll);"),
        "{code}"
    );
    assert!(code.contains("return (-2147483648ll);"), "{code}");
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "T030" && d.severity == Severity::Warning)
    );
}

#[test]
fn wide_numeric_types_and_layouts_reach_hlsl_and_reflection() {
    let compiled = compile(
        "typedef vector<int64_t,2> Pair;struct P{float lead;int64_t scalar;uint64_t3 vector;uint64_t2x3 matrix;};ConstantBuffer<P> p;uint64_t2 f(Pair input){int64_t2 local=int64_t2(1LL,2LL);return uint64_t2(input+local);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(
        code.contains("vector<uint64_t, 2> f(vector<int64_t, 2> input)"),
        "{code}"
    );
    assert!(
        code.contains("matrix<uint64_t, 2, 3> _slang_matrix;"),
        "{code}"
    );
    assert_eq!(compiled.reflection.uniform_types["int64_t"].size, 8);
    assert_eq!(compiled.reflection.uniform_types["uint64_t3"].size, 24);
    assert_eq!(
        compiled.reflection.uniform_types["uint64_t2x3"].matrix_stride,
        Some(16)
    );
    let layout = &compiled.reflection.uniform_types["P"];
    assert_eq!(
        layout.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 8, 16, 48]
    );
    assert_eq!(layout.size, 96);
}

#[test]
fn wide_case_values_convert_to_selector_types_without_aliasing() {
    compile(
        "uint f(uint64_t value){switch(value){case 0xffffffffffffffffULL:return 1;case 0xffffffff:return 2;default:return 3;}}int g(int value){switch(value){case 4294967297LL:return 1;default:return 2;}}",
    );
    let error = compile_source("duplicate.slang", "uint f(uint64_t value){switch(value){case -1LL:return 1;case 0xffffffffffffffffULL:return 2;default:return 3;}}", &CompileOptions::default(), &MemoryIncludes::default()).unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T017"));
}

#[test]
fn wide_values_do_not_enter_unimplemented_32_bit_intrinsics() {
    for source in [
        "uint f(uint64_t x){return countbits(x);}",
        "float f(int64_t x){return asfloat(x);}",
        "groupshared uint64_t value;void f(){InterlockedAdd(value,1ULL);}",
    ] {
        let error = compile_source(
            "unsupported.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T014"),
            "{error}"
        );
    }
}

#[test]
fn suffix_and_magnitude_overflows_remain_source_diagnostics() {
    for (literal, code) in [
        ("1lL", "L004"),
        ("1LlU", "L004"),
        ("1uU", "L004"),
        ("1lul", "L004"),
        ("18446744073709551616ULL", "T010"),
        ("0x10000000000000000", "T010"),
    ] {
        let error = compile_source(
            "overflow.slang",
            &format!("uint64_t f(){{return {literal};}}"),
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == code),
            "{literal}: {error}"
        );
    }
    for literal in [
        "9223372036854775808",
        "18446744073709551615L",
        "+9223372036854775808LL",
        "~9223372036854775808",
    ] {
        let compiled = compile(&format!("uint64_t f(){{return {literal};}}"));
        assert!(
            compiled
                .diagnostics
                .iter()
                .any(|d| d.code == "T030" && d.severity == Severity::Warning),
            "{literal}"
        );
    }
}

#[test]
fn wide_overloads_prefer_lossless_promotions_and_record_mixed_operand_types() {
    let compiled = compile(
        "uint choose(int64_t x){return 1;}uint choose(uint64_t x){return 2;}uint choose(float x){return 3;}uint f(int a,uint b){return choose(a)+choose(b);}bool compare(int64_t a,uint b){return a<b;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("choose__overload_0(a)"), "{code}");
    assert!(code.contains("choose__overload_1(b)"), "{code}");
    assert!(code.contains("a < ((int64_t)b)"), "{code}");
}

#[test]
fn compound_locations_are_evaluated_once_and_right_operands_use_destination_types() {
    let compiled =
        compile("uint f(){uint index=0;uint data[2]={9u,7u};return data[index++]/=-2LL;}");
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("(index++)").count(), 1, "{code}");
    assert!(code.contains("/= ((uint)-2ll)"), "{code}");
}

#[test]
fn mixed_conditional_intrinsic_and_compound_shapes_compile_together() {
    compile(include_str!(
        "../../../tests/conformance/positive/integer64-promotions.slang"
    ));
}

#[test]
fn compound_shifts_keep_independent_integer_count_types() {
    compile(
        "int64_t f(uint shift){int64_t x=1LL;x<<=shift;return x;}int g(uint64_t shift){int x=1;x>>=shift;return x;}",
    );
    let error = compile_source(
        "shift.slang",
        "int f(){int x=1;x<<=2.5;return x;}",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T012"));
}
