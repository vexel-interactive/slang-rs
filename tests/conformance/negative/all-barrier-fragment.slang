[shader("fragment")]
float4 main() : SV_Target
{
    AllMemoryBarrier();
    return float4(1);
}
