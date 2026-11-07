use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "geometry.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{source}: {error}"))
}

#[test]
fn geometric_calls_infer_floating_components_and_preserve_result_shapes() {
    for scalar in [
        Scalar::Int,
        Scalar::Uint,
        Scalar::Half,
        Scalar::Float,
        Scalar::Double,
    ] {
        let mut shapes = vec![Type::Scalar(scalar)];
        shapes.extend((1..=4).map(|n| Type::Vector(scalar, n)));
        for input in shapes {
            let component = if matches!(scalar, Scalar::Int | Scalar::Uint) {
                Scalar::Float
            } else {
                scalar
            };
            let result = input.with_scalar(component);
            let scalar_result = Type::Scalar(component);
            let face_result = Type::Vector(component, input.width().unwrap());
            let compiled = compile(&format!(
                "void run({input} value){{{scalar_result} size=length(value);{scalar_result} gap=distance(value,value);{result} unit=normalize(value);{result} mirror=reflect(value,value);{result} bent=refract(value,value,{scalar_result}(1));{face_result} face=faceforward(value,value,value);}}"
            ));
            let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind
            else {
                panic!()
            };
            for (statement, expected) in statements.iter().zip([
                scalar_result.clone(),
                scalar_result,
                result.clone(),
                result.clone(),
                result,
                face_result,
            ]) {
                let StmtKind::Variable(variable) = &statement.kind else {
                    panic!()
                };
                assert_eq!(
                    variable.initializer.as_ref().unwrap().ty,
                    expected,
                    "{input}: {}",
                    variable.name
                );
            }
        }
    }
}

#[test]
fn cross_preserves_integer_components_and_uses_single_evaluation_helpers() {
    let compiled = compile(
        "static int counter;static int _slang_geometric_cross_0;int3 next(){counter++;return int3(counter);}int3 run(){return cross(next(),next());}int3 again(int3 a,int3 b){return cross(a,b);}",
    );
    let helpers: Vec<_> = compiled
        .ir
        .functions
        .iter()
        .filter(|f| f.name.starts_with("_slang_geometric_"))
        .collect();
    assert_eq!(helpers.len(), 1);
    assert_ne!(helpers[0].name, "_slang_geometric_cross_0");
    assert_eq!(helpers[0].return_type, Type::Vector(Scalar::Int, 3));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((_left).yzx * (_right).zxy)"), "{code}");
    assert!(code.contains("((_left).zxy * (_right).yzx)"), "{code}");
    assert_eq!(code.matches("next(), next()").count(), 1, "{code}");
    assert!(!code.contains("float3"), "{code}");
}

#[test]
fn cross_infers_promotions_splats_and_three_component_results() {
    for (left, right, expected) in [
        ("bool", "bool", Scalar::Int),
        ("bool3", "float3", Scalar::Int),
        ("int1", "int3", Scalar::Int),
        ("uint", "uint3", Scalar::Uint),
        ("int3", "half3", Scalar::Half),
        ("half", "float3", Scalar::Float),
        ("float3", "double3", Scalar::Double),
    ] {
        let compiled = compile(&format!(
            "void run({left} a,{right} b){{{} value=cross(a,b);}}",
            Type::Vector(expected, 3)
        ));
        let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind
        else {
            panic!()
        };
        let StmtKind::Variable(variable) = &statements[0].kind else {
            panic!()
        };
        assert_eq!(
            variable.initializer.as_ref().unwrap().ty,
            Type::Vector(expected, 3)
        );
    }
}

#[test]
fn mixed_floating_shapes_and_scalar_refraction_indices_use_selected_conversions() {
    let code = compile("void run(int3 a,half3 b,double eta){half gap=distance(a,b);half3 mirror=reflect(a,b);double3 bent=refract(a,b,eta);float3 splat=reflect(float3(a),1.0);}").artifact.unwrap();
    assert!(code.contains("distance(((half3)a), b)"), "{code}");
    assert!(code.contains("reflect(((half3)a), b)"), "{code}");
    assert!(code.contains("((float)eta)"), "{code}");
    assert!(code.contains("((float3)1.0)"), "{code}");
}

#[test]
fn double_hlsl_operations_have_explicit_float_inputs_and_double_results() {
    let compiled = compile("double run(double3 a,double3 b){return distance(a,b);}");
    let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(result)) = &statements[0].kind else {
        panic!()
    };
    assert_eq!(result.ty, Type::Scalar(Scalar::Double));
    let ExprKind::Cast { value } = &result.kind else {
        panic!()
    };
    assert_eq!(value.ty, Type::Scalar(Scalar::Float));
    let ExprKind::IntrinsicCall { arguments, .. } = &value.kind else {
        panic!()
    };
    assert!(
        arguments
            .iter()
            .all(|argument| argument.ty == Type::Vector(Scalar::Float, 3))
    );
}

#[test]
fn geometric_defaults_and_imports_keep_declaration_scope() {
    let files = MemoryIncludes { files: [("lib.slang".into(), "module lib;static float3 normal=float3(0,1,0);public float3 mirror(float3 direction=reflect(float3(1),normal)){return direction;}".into())].into() };
    let compiled = compile_source(
        "root.slang",
        "import lib;float3 run(){float3 normal=float3(1,0,0);return mirror();}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    let normal = compiled
        .reflection
        .globals
        .iter()
        .find(|global| global.name == "normal")
        .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(
        code.contains(&format!("reflect(((float3)1), {})", normal.target_name)),
        "{code}"
    );
    assert!(code.contains("mirror(_slang"), "{code}");
}

#[test]
fn geometric_calls_reject_unsupported_shapes_boolean_inputs_and_arities() {
    for source in [
        "void run(){float3 value=cross(float2(1),float2(1));}",
        "void run(){float3 value=cross(float4(1),float4(1));}",
        "void run(){int3 value=cross(bool1(true),bool3(true));}",
        "void run(){float value=distance(float1(1),float3(1));}",
        "void run(){float value=length(float2x2(1));}",
        "void run(){float value=distance(true,1.0);}",
        "void run(){float3 value=reflect(bool3(true),float3(1));}",
        "void run(){float3 value=refract(float3(1),float3(1),float3(1));}",
        "void run(){float3 value=refract(float3(1),float3(1),{0.5});}",
        "void run(){float3 value=faceforward(float3(1),float2(1),float3(1));}",
        "void run(){float value=normalize();}",
        "void run(){float value=length(1.0,2.0);}",
        "void run(){float cross=1;float value=cross(1,2);}",
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
fn geometric_initializer_lists_use_operand_shapes_and_a_scalar_refraction_index() {
    let compiled = compile(
        "void run(float3 value){float gap=distance(value,{0,1,0});float3 product=cross({1,0,0},value);float3 mirror=reflect(value,{0,1,0});float3 bent=refract(value,{0,1,0},0.5);float3 face=faceforward(value,{1,0,0},{0,1,0});}",
    );
    let StmtKind::Block(statements) = &compiled.ir.functions[0].body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Variable(bent) = &statements[3].kind else {
        panic!()
    };
    let ExprKind::IntrinsicCall { arguments, .. } = &bent.initializer.as_ref().unwrap().kind else {
        panic!()
    };
    assert_eq!(arguments[1].ty, Type::Vector(Scalar::Float, 3));
    assert_eq!(arguments[2].ty, Type::Scalar(Scalar::Float));
}

#[test]
fn user_functions_with_geometric_names_keep_the_selected_call() {
    let compiled =
        compile("float cross(float value){return value+1.0;}float run(){return cross(1.0);}");
    let StmtKind::Block(statements) = &compiled.ir.functions[1].body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(result)) = &statements[0].kind else {
        panic!()
    };
    assert!(matches!(result.kind, ExprKind::Call { .. }));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _slang_cross(float value)"), "{code}");
    assert!(code.contains("return _slang_cross(1.0);"), "{code}");
}
