void discardOutside(float2 uv)
{
    if (uv.x < 0.0 || uv.x > 1.0)
    {
        discard;
    }
}

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target0
{
    discardOutside(uv);
    clip(uv.y - 0.5);
    return float4(uv, 0.0, 1.0);
}
