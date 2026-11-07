use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "dot.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{source}: {error}"))
}

#[test]
fn dot_returns_the_selected_scalar_for_every_numeric_shape() {
    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Uint,
        Scalar::Half,
        Scalar::Float,
        Scalar::Double,
    ] {
        let mut inputs = vec![Type::Scalar(scalar)];
        inputs.extend((1..=4).map(|n| Type::Vector(scalar, n)));
        let result = Type::Scalar(if scalar == Scalar::Bool {
            Scalar::Int
        } else {
            scalar
        });
        for input in inputs {
            let compiled = compile(&format!(
                "void run({input} value){{{result} product=dot(value,value);}}"
            ));
            let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind
            else {
                panic!()
            };
            let StmtKind::Variable(variable) = &statements[0].kind else {
                panic!()
            };
            assert_eq!(variable.initializer.as_ref().unwrap().ty, result, "{input}");
        }
    }
}

#[test]
fn integer_and_double_dot_helpers_preserve_arithmetic_and_single_evaluation() {
    let compiled = compile(
        "static int counter;int3 next(){counter++;return int3(counter);}int run(){return dot(next(),next());}uint unsignedDot(uint4 a,uint4 b){return dot(a,b);}double precise(double2 a,double2 b){return dot(a,b);}int again(int3 a,int3 b){return dot(a,b);}",
    );
    let helpers: Vec<_> = compiled
        .ir
        .functions
        .iter()
        .filter(|f| f.name.starts_with("_slang_geometric_dot_"))
        .collect();
    assert_eq!(helpers.len(), 3);
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("next(), next()").count(), 1, "{code}");
    assert!(
        helpers
            .iter()
            .any(|helper| helper.return_type == Type::Scalar(Scalar::Uint))
    );
    assert!(
        helpers
            .iter()
            .any(|helper| helper.return_type == Type::Scalar(Scalar::Double))
    );
    assert!(code.contains("((_left).w * (_right).w)"), "{code}");
    assert!(!code.contains("float"), "{code}");
}

#[test]
fn dot_infers_common_components_scalar_splats_and_boolean_integer_overloads() {
    for (left, right, scalar) in [
        ("int3", "uint3", Scalar::Uint),
        ("int3", "half3", Scalar::Half),
        ("float3", "double3", Scalar::Double),
        ("bool3", "float3", Scalar::Int),
        ("bool3", "uint3", Scalar::Uint),
        ("float3", "float", Scalar::Float),
    ] {
        let result = Type::Scalar(scalar);
        let compiled = compile(&format!(
            "void run({left} a,{right} b){{{result} product=dot(a,b);}}"
        ));
        let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind
        else {
            panic!()
        };
        let StmtKind::Variable(variable) = &statements[0].kind else {
            panic!()
        };
        assert_eq!(variable.initializer.as_ref().unwrap().ty, result);
    }
}

#[test]
fn dot_initializer_lists_and_default_bindings_keep_their_context() {
    let code = compile("static int3 direction=int3(1,2,3);int read(int value=dot(direction,{1,0,0})){return value;}int run(){int3 direction=int3(3,2,1);return read();}").artifact.unwrap();
    assert!(code.contains("(direction, int3(1, 0, 0))"), "{code}");
    assert!(code.contains("read(_slang"), "{code}");
}

#[test]
fn dot_rejects_matrix_aggregate_dimension_and_arity_errors() {
    for source in [
        "void run(){float value=dot(float2x2(1),float2x2(1));}",
        "void run(){float value=dot(float1(1),float3(1));}",
        "void run(){float value=dot(float2(1),float3(1));}",
        "void run(){float value=dot(1.0);}",
        "struct Pair {int x;};void run(){int value=dot(Pair(1),Pair(1));}",
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
    let compiled = compile("float dot(float value){return value;}float run(){return dot(1.0);}");
    let StmtKind::Block(statements) = &compiled.ir.functions[1].body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(result)) = &statements[0].kind else {
        panic!()
    };
    assert!(matches!(result.kind, ExprKind::Call { .. }));
}
