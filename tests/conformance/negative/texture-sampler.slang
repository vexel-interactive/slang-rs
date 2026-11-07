Texture2D<float4> image;
SamplerComparisonState comparison;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    return image.Sample(comparison, uv);
}
