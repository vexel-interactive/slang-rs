void barrier() { DeviceMemoryBarrierWithGroupSync(); }

[shader("vertex")]
float4 main() : SV_Position
{
    barrier();
    return float4(1);
}
