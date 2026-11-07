use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{Expr, ExprKind, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "lists.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn local_initializers(compiled: &slang_compiler::Compilation) -> Vec<&Expr> {
    let function = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &function.body.as_ref().unwrap().kind else {
        panic!()
    };
    body.iter()
        .filter_map(|s| {
            if let StmtKind::Variable(v) = &s.kind {
                v.initializer.as_ref()
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn numeric_lists_distinguish_splats_padding_and_matrix_rows() {
    let compiled = compile(
        "float run(){float a={};float b={2};float3 splat={1};float3 partial={1,2};float2x2 rows={1,2};float2x2 flat={1,2,3};return a+b+splat.z+partial.z+rows[1][0]+flat[1][1];}",
    );
    let values = local_initializers(&compiled);
    assert!(matches!(values[0].kind, ExprKind::Zero));
    assert!(matches!(values[2].kind, ExprKind::Cast { .. }));
    let ExprKind::Construct(parts) = &values[3].kind else {
        panic!()
    };
    assert_eq!(parts.len(), 3);
    assert!(matches!(parts[2].kind, ExprKind::Zero));
    let ExprKind::Construct(rows) = &values[4].kind else {
        panic!()
    };
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| r.ty == Type::Vector(Scalar::Float, 2)
                && matches!(r.kind, ExprKind::Cast { .. }))
    );
    let ExprKind::Construct(rows) = &values[5].kind else {
        panic!()
    };
    let ExprKind::Construct(last) = &rows[1].kind else {
        panic!()
    };
    assert!(matches!(last[1].kind, ExprKind::Zero));
}

#[test]
fn nested_and_flat_lists_keep_struct_defaults_and_memberwise_priority() {
    let compiled = compile(
        "struct P{float z=7;float a=3;};struct S{P p;float x=4;};struct V{float3 v;float x;};float run(){S nested={{2},5};S flat={1,2,3};V memberwise={1,2};V flattened={1,2,3,4};return nested.p.a+flat.x+memberwise.v.z+flattened.x;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((float)3)"));
    assert!(code.contains("((float3)1)"));
    assert!(code.contains("float3(((float)1), ((float)2), ((float)3))"));
}

#[test]
fn array_values_work_in_initializers_assignments_and_arguments() {
    let compiled = compile(
        "typedef float Row[3];static Row global={1};float read(Row a){return a[2];}float run(){Row a={};a={2,3};float grid[2][3]={{1},{2,3,4}};return read({5})+read(a)+grid[1][2]+global[0];}",
    );
    assert_eq!(
        compiled.ir.globals[0].ty,
        Type::Array(Box::new(Type::Scalar(Scalar::Float)), 3)
    );
    assert!(
        compiled
            .ir
            .structures
            .iter()
            .any(|s| s.fields.iter().any(|f| matches!(f.ty, Type::Array(..))))
    );
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("= {}"));
    assert!(code.contains("._value"));
    assert!(code.contains("[2u]"));
    assert_eq!(compiled.reflection.globals[0].type_name, "float[3]");
}

#[test]
fn list_overload_selection_does_not_leak_speculative_types() {
    let compiled = compile(
        "struct P{float x;float y;};float read(float a,int b){return a+b;}float read(P a,float b){return a.y+b;}float pick(float2 a,int b){return a.y+b;}float pick(float3 a,float b){return a.z+b;}float run(){return read({1,2},0.0)+pick({3,4},1);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float2(((float)3), ((float)4))"));
    assert!(!code.contains("float3(((float)3), ((float)4)"));
}

#[test]
fn imported_field_defaults_and_lists_keep_their_owning_module() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;float value(){return 3;}public struct P{public float x;float hidden=value();};public struct S{public P p={2};public float values[2]={4};};public float read(S s){return s.p.hidden+s.values[1];}".into())].into() };
    let compiled = compile_source(
        "main.slang",
        "import lib;float value(){return 9;}float run(){S s={};P p={1};return read(s)+p.x;}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    assert_eq!(compiled.ir.entry_points.len(), 0);
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("_slang__slang_module_1_value()")
    );
}

#[test]
fn numeric_constructor_and_resource_contexts_type_nested_lists() {
    let compiled = compile(
        "Texture2D<float> tex;SamplerState sam;RWByteAddressBuffer raw;float2 next(){return float2(1,2);}float run(){float3 a={next(),3};float3 b=float3({4,5},6);float2x2 m=float2x2({1,2},{3,4});raw.Store2({0},{1,2});return tex.SampleLevel(sam,{0.5,0.5},{0})+dot({1,2},float2(3))+a.z+b.z+m[1][1];}",
    );
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches("next()").count(), 3);
    assert!(code.contains("Store2(((uint)0), uint2(((uint)1), ((uint)2)))"));
}

#[test]
fn invalid_initializer_lists_fail_before_target_generation() {
    for (source, expected) in [
        ("void run(){float x={1,2};}", "T021"),
        ("void run(){float a[2]={1,2,3};}", "T021"),
        ("void run(){float3 v={{1,2,3}};}", "T021"),
        ("void run(){float a[2][2]={{1,2,3},{4}};}", "T021"),
        (
            "float read(float x){return x;}float read(float2 x){return x.x;}float run(){return read({1});}",
            "T014",
        ),
        (
            "void write(out float x){x=1;}void run(){write({1});}",
            "T015",
        ),
        ("float run(){return sin({1});}", "T014"),
        (
            "struct P{float x;float y;};float read(float x){return x;}float read(P x){return x.y;}float run(){return read({1,2});}",
            "T014",
        ),
        (
            "struct P{float x;float y;};void run(){P p={1,2};P copy={p};}",
            "T008",
        ),
        ("void run(){float values[65537]={};}", "T020"),
        ("void run(){float2 x=true?{1,2}:{3,4};}", "T021"),
        ("void run(){float4 x=float4({1},{2});}", "T014"),
        (
            "module m;public struct P{public float x;float hidden;};void run(){P p={1,2};}",
            "T021",
        ),
    ] {
        let error = compile_source(
            "bad.slang",
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
