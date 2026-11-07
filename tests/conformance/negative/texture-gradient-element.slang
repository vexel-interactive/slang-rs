Texture2D<int4> image;
SamplerState imageSampler;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    return float4(image.SampleGrad(imageSampler, uv, float2(0.01), float2(0.01)));
}
