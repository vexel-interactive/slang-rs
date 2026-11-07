use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{ShaderStage, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "shader.slang",
        source,
        &CompileOptions {
            entry_point: Some("main".into()),
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn matrix_algebra_has_dimension_checked_ir() {
    let compiled = compile(include_str!("../../../examples/matrix-transform.slang"));
    let main = &compiled.ir.functions[0];
    let slang_ir::StmtKind::Block(statements) = &main.body.as_ref().unwrap().kind else {
        panic!("expected a block");
    };
    let variables: Vec<_> = statements
        .iter()
        .filter_map(|s| match &s.kind {
            slang_ir::StmtKind::Variable(v) => Some((v.name.as_str(), v.ty.to_string())),
            _ => None,
        })
        .collect();
    assert!(variables.contains(&("transposed", "float3x2".into())));
    assert!(variables.contains(&("product", "float2x2".into())));
    assert!(variables.contains(&("column", "float2".into())));
    assert!(variables.contains(&("row", "float3".into())));
    assert!(variables.contains(&("scaled", "float2x3".into())));
}

#[test]
fn scalar_splat_preserves_a_single_function_call() {
    let compiled = compile(
        "float value(){return 2.0;} float2x2 helper(float x){return float2x2(x);}
         [shader(\"compute\")][numthreads(1,1,1)]void main(){
             float2x2 a=float2x2(value());float2x2 b=helper(2.0);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((float2x2)value())"));
    assert!(code.contains("helper(2.0)"));
    assert_eq!(code.matches("value()").count(), 3); // Prototype, definition, call.
}

#[test]
fn graphics_stages_preserve_semantics_and_reflection() {
    for (source, stage) in [
        (
            include_str!("../../../examples/vertex-transform.slang"),
            ShaderStage::Vertex,
        ),
        (
            include_str!("../../../examples/fragment-color.slang"),
            ShaderStage::Fragment,
        ),
        (
            include_str!("../../../tests/conformance/positive/graphics-out.slang"),
            ShaderStage::Fragment,
        ),
        (
            include_str!("../../../tests/conformance/positive/fragment-integer.slang"),
            ShaderStage::Fragment,
        ),
    ] {
        let compiled = compile(source);
        let entry = &compiled.reflection.entry_points[0];
        assert_eq!(entry.stage, stage);
        assert_eq!(entry.thread_group_size, None);
        assert_eq!(compiled.reflection.schema_version, 7);
        let json = serde_json::to_value(&compiled.reflection).unwrap();
        assert!(json["entry_points"][0].get("thread_group_size").is_none());
        let code = compiled.artifact.unwrap();
        assert!(!code.contains("numthreads"));
        assert!(code.contains("SV_Position"));
    }
}

#[test]
fn invalid_matrix_dimensions_and_graphics_interfaces_fail() {
    for (source, expected) in [
        (
            include_str!("../../../tests/conformance/negative/matrix-product.slang"),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/matrix-constructor.slang"),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/matrix-index.slang"),
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/matrix-determinant.slang"),
            "T014",
        ),
        (
            "[shader(\"vertex\")][numthreads(1,1,1)]float4 main():SV_Position{return float4(0);}",
            "T006",
        ),
        (
            "[shader(\"vertex\")]float2 main():SV_Position{return float2(0);}",
            "T006",
        ),
        (
            "[shader(\"fragment\")]float4 main(uint id:TEXCOORD):SV_Target{return float4(id);}",
            "T006",
        ),
        (
            "[shader(\"fragment\")]float4 main(float a:TEXCOORD,float b:texcoord0):SV_Target{return float4(a+b);}",
            "T006",
        ),
        (
            "struct S {float2 uv;};[shader(\"fragment\")]float4 main(S input):SV_Target{return float4(input.uv,0,1);}",
            "T006",
        ),
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == expected),
            "{source}: {error}"
        );
    }
}

#[test]
fn nested_graphics_structs_and_out_parameters_are_checked() {
    let compiled = compile(
        "struct Position {float4 clip:SV_Position;};
         struct Output {Position position;float2 uv:TEXCOORD;};
         [shader(\"vertex\")]void main(uint id:SV_VertexID,out Output output){
             output.position.clip=float4(float(id),0,0,1);output.uv=float2(0);}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].stage,
        ShaderStage::Vertex
    );
    assert!(matches!(compiled.ir.functions[0].return_type, Type::Void));
    let compute = compile(include_str!("../../../examples/vector-add.slang"));
    let json = serde_json::to_value(&compute.reflection).unwrap();
    assert_eq!(
        json["entry_points"][0]["thread_group_size"],
        serde_json::json!([64, 1, 1])
    );
}
