use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, Severity, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> Compilation {
    compile_source(
        "pointer.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn pointer_types_and_core_aliases_keep_distinct_source_identities() {
    for (name, scalar) in [
        ("intptr_t", Scalar::IntPtr),
        ("ssize_t", Scalar::IntPtr),
        ("uintptr_t", Scalar::UintPtr),
        ("size_t", Scalar::UintPtr),
        ("usize_t", Scalar::UintPtr),
    ] {
        assert_eq!(Type::builtin(name), Some(Type::Scalar(scalar)));
    }
    for name in ["intptr_t2", "uintptr_t4", "intptr_t2x2", "size_t2"] {
        assert_eq!(Type::builtin(name), None);
    }
    let compiled = compile(
        "intptr_t signedValue;uintptr_t unsignedValue;int64_t fixedValue;vector<intptr_t,2> pair;",
    );
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .map(|g| g.type_name.as_str())
            .collect::<Vec<_>>(),
        ["intptr_t", "uintptr_t", "int64_t", "vector<intptr_t, 2>"]
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("int64_t signedValue;"), "{code}");
    assert!(code.contains("uint64_t unsignedValue;"), "{code}");
    assert!(code.contains("vector<int64_t, 2> pair;"), "{code}");
}

#[test]
fn pointer_constants_preserve_high_bits_signed_shifts_and_forward_dimensions() {
    let compiled = compile(
        "static const intptr_t Min=-9223372036854775808Z;static const intptr_t Hex=0xffffffffffffffffZ;static const uintptr_t Max=0xffffffffffffffffUZ;static const uintptr_t Quotient=Max/3UZ;static const intptr_t Shift=Min>>62;float data[Count];static const intptr_t Count=(1Z<<40)>>39;",
    );
    assert_eq!(compiled.reflection.globals[5].type_name, "float[2]");
    let code = compiled.artifact.unwrap();
    for text in [
        "Min = (-9223372036854775807ll - 1ll);",
        "Hex = -1ll;",
        "Max = 18446744073709551615ull;",
        "Quotient = 6148914691236517205ull;",
        "Shift = -2ll;",
    ] {
        assert!(code.contains(text), "{text}: {code}");
    }
}

#[test]
fn pointer_shapes_have_hlsl_storage_and_independent_reflection_layouts() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/integer-pointer-storage.slang"
    ));
    let layout = &compiled.reflection.uniform_types["Params"];
    assert_eq!(layout.size, 120);
    assert_eq!(
        layout.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 8, 16, 48, 96]
    );
    assert_eq!(compiled.reflection.uniform_types["intptr_t"].size, 8);
    assert_eq!(
        compiled.reflection.uniform_types["vector<uintptr_t, 3>"].size,
        24
    );
    assert_eq!(
        compiled.reflection.uniform_types["matrix<uintptr_t, 2, 3>"].matrix_stride,
        Some(16)
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("intptr_t"), "{code}");
    assert!(!code.contains("uintptr_t"), "{code}");
    assert!(
        code.contains("StructuredBuffer<vector<int64_t, 2> >"),
        "{code}"
    );
}

#[test]
fn pointer_constructors_defaults_and_dimensions_reach_every_context() {
    let compiled = compile(
        "typedef vector<intptr_t,2> Pair;static const uintptr_t Count=1UZ;Pair read(Pair value=Pair(1Z,2Z)){return value;}RWStructuredBuffer<uintptr_t> output;[shader(\"compute\")][numthreads(Count,1Z,1ZU)]void main(){Pair local=vector<intptr_t,2>(3Z);Pair value=read();output[0]=uintptr_t(local.x+value.y);}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([1, 1, 1])
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("vector<int64_t, 2>(1ll, 2ll)"), "{code}");
    assert!(code.contains("((vector<int64_t, 2>)3ll)"), "{code}");
}

#[test]
fn pointer_literal_overflow_and_suffix_diagnostics_remain_distinct() {
    for (literal, expected) in [
        ("1ZL", "L004"),
        ("1ZZ", "L004"),
        ("1UZU", "L004"),
        ("18446744073709551616UZ", "T010"),
    ] {
        let error = compile_source(
            "bad.slang",
            &format!("uintptr_t f(){{return {literal};}}"),
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == expected),
            "{error}"
        );
    }
    let compiled = compile(
        "uintptr_t f(){return 9223372036854775808Z;}intptr_t g(){return -9223372036854775808Z;}",
    );
    assert_eq!(
        compiled
            .diagnostics
            .iter()
            .filter(|d| d.code == "T030" && d.severity == Severity::Warning)
            .count(),
        1
    );
}

#[test]
fn pointer_switch_labels_normalize_to_the_selector_without_truncation() {
    compile(
        "uint f(intptr_t value){switch(value){case 0xffffffffffffffffZ:return 1;case 0xffffffff:return 2;default:return 3;}}",
    );
    let error = compile_source("bad.slang", "uint f(uintptr_t value){switch(value){case -1Z:return 1;case 0xffffffffffffffffUZ:return 2;default:return 3;}}",
        &CompileOptions::default(), &MemoryIncludes::default()).unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T017"));
}

#[test]
fn pointer_arithmetic_generic_intrinsics_and_bit_operations_compile_together() {
    for source in [
        include_str!("../../../tests/conformance/positive/integer-pointer-literals.slang"),
        include_str!("../../../tests/conformance/positive/integer-pointer-promotions.slang"),
        include_str!("../../../tests/conformance/positive/integer-pointer-intrinsics.slang"),
        include_str!("../../../tests/conformance/positive/integer-pointer-bitops.slang"),
        include_str!("../../../tests/conformance/positive/integer-pointer-bitops-buffer.slang"),
        include_str!("../../../tests/conformance/positive/integer-pointer-contexts.slang"),
    ] {
        compile(source);
    }
}

#[test]
fn pointer_bitcasts_follow_selected_32_bit_overload_conversions() {
    let code = compile("float literal(){return asfloat(1Z);}float variable(intptr_t x){return asfloat(x);}uint bits(uintptr_t x){return asuint(x);}").artifact.unwrap();
    assert!(code.contains("asfloat(((int)1ll))"), "{code}");
    assert!(code.contains("return ((float)x);"), "{code}");
    assert!(code.contains("asuint(((float)x))"), "{code}");
    let code = compile("float f(int64_t x){return asfloat(x);}")
        .artifact
        .unwrap();
    assert!(code.contains("return ((float)x);"), "{code}");
}

#[test]
fn literal_rank_ties_do_not_choose_an_arbitrary_pointer_overload() {
    let error = compile_source("bad.slang", "uint choose(intptr_t x){return 1;}uint choose(int64_t x){return 2;}uint f(){return choose(1);}",
        &CompileOptions::default(), &MemoryIncludes::default()).unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "T014" && d.message.contains("ambiguous"))
    );
}

#[test]
fn pointer_matrix_intrinsics_require_matrix_arguments() {
    let error = compile_source(
        "bad.slang",
        include_str!("../../../tests/conformance/negative/integer-pointer-matrix-intrinsic.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T014"));
}
