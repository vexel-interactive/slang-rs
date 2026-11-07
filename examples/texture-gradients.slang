Texture2D<float4> image;
SamplerState imageSampler;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    float2 dx = float2(0.01, 0.0);
    float2 dy = float2(0.0, 0.01);
    float4 gradient = image.SampleGrad(imageSampler, uv, dx, dy);
    float4 biased = image.SampleBias(imageSampler, uv, 1.0);
    return lerp(gradient, biased, 0.5);
}
