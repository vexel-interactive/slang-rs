float readShared() { GroupMemoryBarrier(); return 1; }
float sample(float value = readShared()) { return value; }

[shader("fragment")]
float4 main() : SV_Target
{
    return float4(sample());
}
