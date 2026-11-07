Texture2D<float> depth;
TextureCube<float4> environment;
SamplerComparisonState depthSampler;
SamplerState environmentSampler;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    float visible = depth.SampleCmp(depthSampler, uv, 0.5);
    float base = depth.SampleCmpLevelZero(depthSampler, uv, 0.5);
    float4 color = environment.SampleLevel(environmentSampler, float3(uv, 1.0), 0.0);
    return color * (visible + base) / 2.0;
}
