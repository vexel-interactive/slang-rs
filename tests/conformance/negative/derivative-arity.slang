[shader("fragment")]
float4 main() : SV_Target
{
    return float4(fwidth_fine(1.0, 2.0));
}
