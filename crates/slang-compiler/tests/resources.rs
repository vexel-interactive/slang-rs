use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn hlsl_names_remain_distinct_and_reflect_target_entry_names() {
    let compiled = compile_source(
        "names.slang",
        include_str!("../../../tests/conformance/positive/hlsl-identifiers.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("struct _slang_point"));
    assert!(code.contains("_slang_triangle"));
    assert_eq!(compiled.reflection.globals[0].target_name, "_slang_line");
    assert_eq!(
        compiled.reflection.globals[1].target_name,
        "_slang__slang_line"
    );
    let compiled = compile_source(
        "entry.slang",
        "[shader(\"compute\")][numthreads(1,1,1)]void line(){}",
        &CompileOptions {
            entry_point: Some("line".into()),
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(compiled.reflection.entry_points[0].name, "line");
    assert_eq!(
        compiled.reflection.entry_points[0].target_name,
        "_slang_line"
    );
    assert!(compiled.artifact.unwrap().contains("void _slang_line()"));
}

#[test]
fn resource_methods_lower_receivers_and_preserve_output_arguments() {
    let compiled = compile_source(
        "load-store.slang",
        include_str!("../../../tests/conformance/positive/resource-load-store.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("(values).GetDimensions(count, stride)"));
    assert!(code.contains("(volume).Load(int4(0, 0, 0, 0))"));
    assert_eq!(
        compiled.reflection.globals[0].resource_access.as_deref(),
        Some("read")
    );
    assert_eq!(
        compiled.reflection.globals[2].resource_access.as_deref(),
        Some("read_write")
    );
    let json = serde_json::to_value(&compiled.ir).unwrap();
    assert_eq!(
        json["functions"][0]["body"]["kind"]["Block"][2]["kind"]["Expression"]["kind"]["MethodCall"]
            ["receiver"]["kind"]["Name"],
        "values"
    );
}

#[test]
fn sampled_and_comparison_resources_compile_independently() {
    for source in [
        include_str!("../../../examples/texture-sample.slang"),
        include_str!("../../../tests/conformance/positive/texture-comparison.slang"),
        "Texture2D image;SamplerState samplerState;
         [shader(\"fragment\")]float4 main(float2 uv:TEXCOORD):SV_Target{return image.Sample(samplerState,uv);}",
        "struct Material {Texture2D<float4> color;};Material material;SamplerState samplerState;
         [shader(\"fragment\")]float4 main(float2 uv:TEXCOORD):SV_Target{return material.color.Sample(samplerState,uv);}",
    ] {
        assert!(compile_source("sample.slang", source, &CompileOptions::default(), &MemoryIncludes::default()).is_ok());
    }
}

#[test]
fn invalid_resource_access_and_method_arguments_are_rejected() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/read-only-texture.slang"),
            "T015",
        ),
        (
            include_str!("../../../tests/conformance/negative/texture-coordinate.slang"),
            "T013",
        ),
        (
            include_str!("../../../tests/conformance/negative/texture-sampler.slang"),
            "T008",
        ),
        (
            include_str!("../../../tests/conformance/negative/texture-output-argument.slang"),
            "T015",
        ),
        ("Texture2D<bool> image;", "T002"),
        (
            "RWTexture2D<float4> image;SamplerState s;float4 f(){return image.Sample(s,float2(0));}",
            "T014",
        ),
        (
            "Texture3D<float> image;SamplerComparisonState s;float f(){return image.SampleCmp(s,float3(0),0.5);}",
            "T014",
        ),
        (
            "Texture2D<float4> image;void f(){uint w;int h;image.GetDimensions(w,h);}",
            "T014",
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
            error.diagnostics.iter().any(|d| d.code == code),
            "{source}: {error}"
        );
    }
}
