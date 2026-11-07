use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "derivatives.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
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
fn derivatives_preserve_all_floating_scalar_vector_and_matrix_shapes() {
    for scalar in [Scalar::Half, Scalar::Float, Scalar::Double] {
        let mut types = vec![Type::Scalar(scalar)];
        types.extend((2..=4).map(|width| Type::Vector(scalar, width)));
        for rows in 2..=4 {
            types.extend((2..=4).map(|columns| Type::Matrix(scalar, rows, columns)));
        }
        for ty in types {
            let calls = [
                "ddx",
                "ddy",
                "ddx_coarse",
                "ddy_coarse",
                "ddx_fine",
                "ddy_fine",
                "fwidth",
                "fwidth_coarse",
                "fwidth_fine",
            ]
            .map(|name| format!("{ty} v_{name}={name}(value);"))
            .join("");
            let source = format!("void run({ty} value){{{calls}}}");
            let compiled = compile(&source);
            let run = compiled
                .ir
                .functions
                .iter()
                .find(|f| f.name == "run")
                .unwrap();
            let slang_ir::StmtKind::Block(statements) = &run.body.as_ref().unwrap().kind else {
                panic!()
            };
            for statement in statements {
                let slang_ir::StmtKind::Variable(variable) = &statement.kind else {
                    panic!()
                };
                assert_eq!(variable.ty, ty);
                assert_eq!(variable.initializer.as_ref().unwrap().ty, ty);
            }
        }
    }
}

#[test]
fn integer_derivatives_select_float_shapes_and_explicit_input_conversions() {
    let compiled = compile(
        "float run(int x,uint2 y,int2x3 z){float a=ddx(x);float2 b=ddy(y);float2x3 c=fwidth_fine(z);return a+b.x+c[0][0];}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("ddx(((float)x))"));
    assert!(code.contains("ddy(((float2)y))"));
    assert!(code.contains("((float2x3)z)"));
}

#[test]
fn coarse_and_fine_width_helpers_evaluate_arguments_once_and_are_cached() {
    let compiled = compile(
        "static float counter;static float _slang_derivative_0;float next(){counter+=1;return counter;}float run(){float a=fwidth_coarse(next());float b=fwidth_coarse(next());return a+b+fwidth_fine(next());}",
    );
    let helpers: Vec<_> = compiled
        .ir
        .functions
        .iter()
        .filter(|f| f.name.starts_with("_slang_derivative__"))
        .collect();
    assert_eq!(helpers.len(), 2);
    assert!(helpers.iter().all(|f| f.parameters.len() == 1));
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("next()").count(), 5); // Prototype, definition, three evaluations.
    assert!(code.contains("ddx_coarse(_input)"));
    assert!(code.contains("ddy_fine(_input)"));
}

#[test]
fn selected_overloads_and_supplied_defaults_do_not_require_derivatives() {
    let source = "float read(){return ddx(1.0);}float sample(float x=read()){return x;}struct P{float x=read();};void pick(float x){ddy(x);}void pick(int x){}[shader(\"compute\")][numthreads(1,1,1)]void main(){pick(1);P p=P(2);float value=sample(3);}";
    let compiled = compile(source);
    assert_eq!(
        compiled.reflection.entry_points[0]
            .target_profile
            .as_deref(),
        Some("cs_6_0")
    );
    fails(
        "float read(){return ddx(1.0);}float sample(float x=read()){return x;}[shader(\"compute\")][numthreads(1,1,1)]void main(){float value=sample();}",
        "T024",
    );
}

#[test]
fn compute_derivatives_require_valid_quad_dimensions() {
    for dimensions in ["4,1,1", "8,1,1", "2,2,1", "2,2,2", "4,4,2"] {
        let source = format!(
            "[shader(\"compute\")][numthreads({dimensions})]void main(){{float value=ddx(1.0);}}"
        );
        let compiled = compile(&source);
        assert_eq!(
            compiled.reflection.entry_points[0]
                .target_profile
                .as_deref(),
            Some("cs_6_6")
        );
    }
    for dimensions in ["1,1,1", "2,1,1", "4,1,2", "3,2,1"] {
        fails(
            &format!(
                "[shader(\"compute\")][numthreads({dimensions})]void main(){{float value=ddy(1.0);}}"
            ),
            "T024",
        );
    }
}

#[test]
fn vertex_derivative_requirements_follow_defaults_globals_and_recursive_calls() {
    for source in [
        "[shader(\"vertex\")]float4 main():SV_Position{return float4(ddx(1.0));}",
        "float read(){return fwidth_fine(1.0);}static float value=read();[shader(\"vertex\")]float4 main():SV_Position{return float4(value);}",
        "float read(){return ddx(1.0);}struct P{float x=read();};[shader(\"vertex\")]float4 main():SV_Position{P p={};return float4(p.x);}",
        "void a(){b();}void b(){a();ddy(1.0);}[shader(\"vertex\")]float4 main():SV_Position{a();return float4(1);}",
    ] {
        fails(source, "T023");
    }
}

#[test]
fn imported_derivatives_keep_builtin_selection_and_diagnostic_locations() {
    let files = MemoryIncludes {
        files: [(
            "math.slang".into(),
            "module math;public float width(float x){return fwidth_coarse(x);}".into(),
        )]
        .into(),
    };
    let source = "import math;float fwidth_coarse(float x){return x;}[shader(\"fragment\")]float4 main(float2 uv:TEXCOORD0):SV_Target{return float4(width(uv.x)+fwidth_coarse(uv.y));}";
    let compiled =
        compile_source("main.slang", source, &CompileOptions::default(), &files).unwrap();
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("float _slang_fwidth_coarse(float x)")
    );
    let error = compile_source(
        "main.slang",
        "import math;[shader(\"vertex\")]float4 main():SV_Position{return float4(width(1));}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap_err();
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "T023").unwrap();
    assert!(
        error
            .sources
            .get(diagnostic.span.source)
            .unwrap()
            .name
            .ends_with("math.slang")
    );
}

#[test]
fn invalid_derivative_arguments_are_rejected() {
    for source in [
        "void run(){ddx();}",
        "void run(){ddy(1,2);}",
        "void run(){fwidth(true);}",
        "void run(){fwidth_coarse(bool2(true));}",
        "void run(){fwidth_fine({});}",
        "struct P{float x;};void run(){ddx(P(1));}",
    ] {
        fails(source, "T014");
    }
}
