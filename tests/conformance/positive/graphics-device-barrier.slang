RWStructuredBuffer<uint> result;

[shader("fragment")]
float4 main() : SV_Target
{
    result[0] = 1u;
    DeviceMemoryBarrier();
    return float4(result[0]);
}
