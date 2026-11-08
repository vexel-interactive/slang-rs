use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "bits.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{source}: {error}"))
}

#[test]
fn integer_bit_functions_preserve_signedness_width_and_unsigned_result_types() {
    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Uint,
        Scalar::Int64,
        Scalar::Uint64,
    ] {
        let mut types = vec![Type::Scalar(scalar)];
        types.extend((1..=4).map(|n| Type::Vector(scalar, n)));
        for ty in types {
            let parameter = ty.with_scalar(if scalar == Scalar::Bool {
                Scalar::Int
            } else {
                scalar
            });
            let unsigned = ty.with_scalar(Scalar::Uint);
            let compiled = compile(&format!(
                "void run({ty} input){{{unsigned} count=countbits(input);{unsigned} high=firstbithigh(input);{unsigned} low=firstbitlow(input);{parameter} reversed=reversebits(input);}}"
            ));
            let slang_ir::StmtKind::Block(statements) =
                &compiled.ir.functions[0].body.as_ref().unwrap().kind
            else {
                panic!()
            };
            for (statement, expected) in
                statements
                    .iter()
                    .zip([unsigned.clone(), unsigned.clone(), unsigned, parameter])
            {
                let slang_ir::StmtKind::Variable(variable) = &statement.kind else {
                    panic!()
                };
                assert_eq!(variable.initializer.as_ref().unwrap().ty, expected);
            }
        }
    }
}

#[test]
fn wide_bit_operations_retain_values_types_and_single_evaluation() {
    compile(include_str!(
        "../../../tests/conformance/positive/integer64-bitops.slang"
    ));
    let code = compile("uint run(inout uint64_t value){return countbits(value++);}")
        .artifact
        .unwrap();
    assert_eq!(code.matches("(value++)").count(), 1, "{code}");
    assert!(code.contains("countbits((value++))"), "{code}");
}

#[test]
fn signed_inputs_and_boolean_promotions_remain_explicit() {
    let code = compile("void run(int value,bool2 flag){uint high=firstbithigh(value);int reversed=reversebits(value);uint2 counts=countbits(flag);}").artifact.unwrap();
    assert!(code.contains("firstbithigh(value)"), "{code}");
    assert!(code.contains("reversebits(value)"), "{code}");
    assert!(code.contains("countbits(((int2)flag))"), "{code}");
    assert!(!code.contains("firstbithigh(((uint)value))"));
}

#[test]
fn integer_bit_functions_reject_floating_aggregate_and_arity_inputs() {
    for source in [
        "void run(){uint value=countbits(1.0);}",
        "void run(){uint value=firstbitlow(double(1));}",
        "void run(){int2x2 input=1;uint2x2 value=countbits(input);}",
        "void run(){uint value=reversebits();}",
        "void run(){uint value=firstbithigh(1,2);}",
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T014"),
            "{source}: {error}"
        );
    }
}

#[test]
fn bit_operations_in_defaults_use_their_declaration_bindings() {
    let code = compile("static uint mask=15u;uint read(uint value=countbits(mask)){return value;}uint run(){uint mask=1u;return read();}").artifact.unwrap();
    assert!(code.contains("return countbits(mask);"), "{code}");
    assert!(code.contains("read(_slang"), "{code}");
}
