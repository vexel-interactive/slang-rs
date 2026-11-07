[shader("fragment")]
float4 main() : SV_Target
{
    return float4(ddx(true));
}
