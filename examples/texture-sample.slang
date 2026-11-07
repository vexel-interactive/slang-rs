Texture2D<float4> image;
SamplerState imageSampler;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    uint width;
    uint height;
    uint levels;
    image.GetDimensions(0u, width, height, levels);
    float2 texel = float2(1.0 / float(width), 1.0 / float(height));
    float4 filtered = image.Sample(imageSampler, uv);
    float4 explicitLod = image.SampleLevel(imageSampler, uv + texel, 0.0);
    float4 first = image.Load(int3(0, 0, 0));
    return (filtered + explicitLod + first) / 3.0;
}
