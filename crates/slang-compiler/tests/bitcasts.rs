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

fn fails(source: &str, code: &str) {
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

#[test]
fn bitcasts_preserve_each_supported_32_bit_scalar_vector_and_matrix_shape() {
    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Uint,
        Scalar::Half,
        Scalar::Float,
    ] {
        let mut types = vec![Type::Scalar(scalar)];
        types.extend((1..=4).map(|n| Type::Vector(scalar, n)));
        for rows in 1..=4 {
            types.extend((1..=4).map(|columns| Type::Matrix(scalar, rows, columns)));
        }
        for ty in types {
            let compiled = compile(&format!(
                "void run({ty} input){{{} integer=asint(input);{} unsignedValue=asuint(input);{} floating=asfloat(input);}}",
                ty.with_scalar(Scalar::Int),
                ty.with_scalar(Scalar::Uint),
                ty.with_scalar(Scalar::Float)
            ));
            let run = compiled
                .ir
                .functions
                .iter()
                .find(|f| f.name == "run")
                .unwrap();
            let slang_ir::StmtKind::Block(statements) = &run.body.as_ref().unwrap().kind else {
                panic!()
            };
            for (statement, target) in
                statements
                    .iter()
                    .zip([Scalar::Int, Scalar::Uint, Scalar::Float])
            {
                let slang_ir::StmtKind::Variable(variable) = &statement.kind else {
                    panic!()
                };
                assert_eq!(
                    variable.initializer.as_ref().unwrap().ty,
                    ty.with_scalar(target)
                );
            }
        }
    }
}

#[test]
fn bitcast_promotions_are_explicit_and_identity_calls_evaluate_once() {
    let compiled = compile(
        "static int count;int next(){count+=1;return count;}void run(bool2 flag,half2 input){int value=asint(next());float2 first=asfloat(flag);uint2 second=asuint(input);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("int value = next();"), "{code}");
    assert!(code.contains("asfloat(((int2)flag))"), "{code}");
    assert!(code.contains("asuint(((float2)input))"), "{code}");
    assert!(!code.contains("asint(next())"));
}

#[test]
fn double_words_split_into_writable_integer_locations_without_output_casts() {
    let compiled = compile(
        "RWStructuredBuffer<uint> words;void run(double input){int high;asuint(input,words[0],high);double packed=asdouble(words[0],high);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("asuint(input, (words)[0], high)"), "{code}");
    assert!(
        code.contains("asdouble((words)[0], ((uint)high))"),
        "{code}"
    );
    compile("void run(float input){uint low,high;asuint(input,low,high);}");
    fails(
        "void run(double input){const uint low=0;uint high;asuint(input,low,high);}",
        "T015",
    );
    fails(
        "void run(double input){uint high;asuint(input,0u,high);}",
        "T015",
    );
}

#[test]
fn double_packing_selects_scalar_or_two_component_signatures() {
    let compiled = compile(
        "void run(uint1 a,uint2 b,int c){double scalar=asdouble(a,a);double2 vector=asdouble(b,c);}",
    );
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let slang_ir::StmtKind::Block(statements) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    for (statement, ty) in statements.iter().zip([
        Type::Scalar(Scalar::Double),
        Type::Vector(Scalar::Double, 2),
    ]) {
        let slang_ir::StmtKind::Variable(variable) = &statement.kind else {
            panic!()
        };
        assert_eq!(variable.initializer.as_ref().unwrap().ty, ty);
    }
    let code = compiled.artifact.unwrap();
    assert!(code.contains("asdouble(((uint)a), ((uint)a))"), "{code}");
    assert!(code.contains("asdouble(b, ((uint2)c))"), "{code}");
}

#[test]
fn unsupported_bitcast_signatures_are_diagnostics() {
    for source in [
        "void run(){uint value=asuint(double(1));}",
        "void run(){float value=asfloat(double(1));}",
        "void run(){float value=asfloat();}",
        "void run(){float value=asfloat(1,2);}",
        "void run(){uint values[2];float value=asfloat(values);}",
        "void run(double2 input){uint2 low,high;asuint(input,low,high);}",
        "void run(double input){float low,high;asuint(input,low,high);}",
        "void run(uint3 input){double3 value=asdouble(input,input);}",
    ] {
        fails(source, "T014");
    }
}

#[test]
fn user_functions_with_bitcast_names_retain_their_own_semantics() {
    let code =
        compile("float asfloat(int value){return float(value);}float run(){return asfloat(4);}")
            .artifact
            .unwrap();
    assert!(code.contains("float _slang_asfloat(int value)"), "{code}");
    assert!(code.contains("return _slang_asfloat(4);"), "{code}");
}
