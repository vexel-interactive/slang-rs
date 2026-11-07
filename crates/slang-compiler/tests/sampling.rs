use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "sampling.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn bias_and_gradient_methods_preserve_texture_element_types_and_dimensions() {
    let compiled = compile(
        "Texture1D<float> t1;Texture2D<float4> t2;Texture3D<half2> t3;TextureCube<float3> tc;SamplerState state;
        float read1(float uv){return t1.SampleGrad(state,uv,1.0,1.0)+t1.SampleBias(state,uv,0.0);}
        float4 read2(float2 uv){return t2.SampleGrad(state,uv,float2(1),float2(1))+t2.SampleBias(state,uv,0.0);}
        half2 read3(float3 uv){return t3.SampleGrad(state,uv,float3(1),float3(1))+t3.SampleBias(state,uv,0.0);}
        float3 readCube(float3 uv){return tc.SampleGrad(state,uv,float3(1),float3(1))+tc.SampleBias(state,uv,0.0);}",
    );
    let types: Vec<_> = compiled
        .ir
        .functions
        .iter()
        .map(|f| f.return_type.to_string())
        .collect();
    assert_eq!(types, ["float", "float4", "half2", "float3"]);
    let code = compiled.artifact.unwrap();
    assert_eq!(code.matches(".SampleGrad(").count(), 4);
    assert_eq!(code.matches(".SampleBias(").count(), 4);
}

#[test]
fn resource_signatures_contextually_type_gradient_lists() {
    compile(
        "Texture2D<float4> texture;SamplerState state;float4 read(){return texture.SampleGrad(state,{0.0,0.0},{1.0,0.0},{0.0,1.0})+texture.SampleBias(state,{0.0,0.0},1);}",
    );
    compile(
        "Texture2D<float4> texture;SamplerState state;RWStructuredBuffer<float4> result;[shader(\"compute\")][numthreads(1,1,1)]void main(){result[0]=texture.SampleGrad(state,float2(0),float2(1),float2(1));}",
    );
}

#[test]
fn unsupported_sampling_receivers_and_invalid_gradients_are_rejected() {
    for (source, code) in [
        (
            "Texture2D<int4> t;SamplerState s;int4 run(){return t.SampleGrad(s,float2(0),float2(1),float2(1));}",
            "T014",
        ),
        (
            "RWTexture2D<float4> t;SamplerState s;float4 run(){return t.SampleBias(s,float2(0),1.0);}",
            "T014",
        ),
        (
            "Texture2D<float4> t;SamplerComparisonState s;float4 run(){return t.SampleGrad(s,float2(0),float2(1),float2(1));}",
            "T008",
        ),
        (
            "Texture2D<float4> t;SamplerState s;float4 run(){return t.SampleGrad(s,float2(0),float3(1),float2(1));}",
            "T008",
        ),
        (
            "TextureCube<float4> t;SamplerState s;float4 run(){return t.SampleGrad(s,float3(0),float2(1),float3(1));}",
            "T008",
        ),
        (
            "Texture2D<float4> t;SamplerState s;float4 run(){return t.SampleGrad(s,float2(0),float2(1));}",
            "T014",
        ),
        (
            "Texture2D<float4> t;SamplerState s;float4 run(){return t.SampleBias(s,float2(0),1.0,int2(1));}",
            "T014",
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
