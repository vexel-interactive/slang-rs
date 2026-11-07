[shader("fragment")]
float4 main() : SV_Target
{
    GroupMemoryBarrierWithGroupSync();
    return float4(1);
}
