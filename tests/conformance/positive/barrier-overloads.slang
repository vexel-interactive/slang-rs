void barrier(float value)
{
    GroupMemoryBarrierWithGroupSync();
}

void barrier(int value) {}
void GroupMemoryBarrier() {}

[shader("fragment")]
float4 main() : SV_Target
{
    barrier(1);
    GroupMemoryBarrier();
    return float4(1);
}
