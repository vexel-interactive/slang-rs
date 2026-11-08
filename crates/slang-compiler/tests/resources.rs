use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

#[test]
fn byte_buffers_preserve_widths_atomic_outputs_and_reflection() {
    let compiled = compile_source(
        "raw.slang",
        include_str!("../../../examples/byte-address-buffer.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "ByteAddressBuffer"
    );
    assert_eq!(
        compiled.reflection.globals[0].resource_access.as_deref(),
        Some("read")
    );
    assert_eq!(
        compiled.reflection.globals[1].type_name,
        "RWByteAddressBuffer"
    );
    assert_eq!(
        compiled.reflection.globals[1].resource_access.as_deref(),
        Some("read_write")
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("uint4 values = (input).Load4(0u)"));
    assert!(code.contains("(output).InterlockedAdd(16u, byteCount, original)"));
    for (source, code) in [
        (
            "ByteAddressBuffer data;void run(){data.Store(0u,1u);}",
            "T014",
        ),
        (
            "RWByteAddressBuffer data;void run(){data.InterlockedAdd(0u,1u,0u);}",
            "T015",
        ),
        (
            "RWByteAddressBuffer data;void run(){float result;data.InterlockedAdd(0u,1u,result);}",
            "T014",
        ),
        (
            "RWByteAddressBuffer data;void run(){uint2 count;data.GetDimensions(count);}",
            "T014",
        ),
        (
            "ByteAddressBuffer data;void run(){uint x=data.Load3(0u);}",
            "T008",
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
            error.diagnostics.iter().any(|d| d.code == code),
            "{source}: {error}"
        );
    }
}

#[test]
fn constant_buffer_fields_are_typed_and_read_only() {
    let compiled = compile_source(
        "uniforms.slang",
        include_str!("../../../examples/constant-buffer.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "ConstantBuffer<Parameters>"
    );
    assert_eq!(
        compiled.reflection.globals[0].resource_access.as_deref(),
        Some("read")
    );
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("ConstantBuffer<Parameters> parameters")
    );
    for source in [
        "struct P{float2 value;};ConstantBuffer<P> p;void run(){p.value.x=1;}",
        "struct P{float value[2];};ConstantBuffer<P> p;void run(){p.value[0]=1;}",
        "struct P{float value;};ConstantBuffer<P> p;void store(out float x){x=1;}void run(){store(p.value);}",
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T015"),
            "{error}"
        );
    }
    let error = compile_source(
        "unsupported.slang",
        "ConstantBuffer<float> p;",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T002"));
    for source in [
        "struct Inner{ByteAddressBuffer data;};struct Outer{Inner inner;};ConstantBuffer<Outer> p;",
        "struct P{Texture2D<float> texture;};void run(ConstantBuffer<P> p){}",
        "struct P{SamplerState sampler;};void run(){ConstantBuffer<P> p;}",
    ] {
        let error = compile_source(
            "unsupported.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T002"),
            "{error}"
        );
    }
}

#[test]
fn imported_constant_buffer_types_preserve_visibility_and_hlsl_names() {
    let files=MemoryIncludes {files:[("lib.slang".into(),
        "module lib;public struct Parameters{public float value;float hidden;};public ConstantBuffer<Parameters> parameters;".into())].into()};
    let compiled = compile_source(
        "main.slang",
        "import lib;float run(){return parameters.value;}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.globals[0].module.as_deref(),
        Some("lib")
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("ConstantBuffer<_slang__slang_module_1_Parameters>"));
    let error = compile_source(
        "main.slang",
        "import lib;float run(){return parameters.hidden;}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T018"));
}

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
            ["receiver"]["kind"]["GlobalName"],
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
