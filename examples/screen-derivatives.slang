[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target
{
    float2 horizontal = ddx(uv);
    float2 vertical = ddy(uv);
    float2 width = fwidth(uv);
    float2 coarse = fwidth_coarse(uv);
    float2 fine = fwidth_fine(uv);
    return float4(horizontal + vertical + width + coarse + fine, 0, 1);
}
