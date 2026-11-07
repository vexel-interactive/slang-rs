RWStructuredBuffer<uint> result;
groupshared uint values[4];

[shader("compute")]
[numthreads(4, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    values[index] = index;
    GroupMemoryBarrier();
    GroupMemoryBarrierWithGroupSync();
    result[index] = values[(index + 1u) & 3u];
    DeviceMemoryBarrier();
    DeviceMemoryBarrierWithGroupSync();
    AllMemoryBarrier();
    AllMemoryBarrierWithGroupSync();
}
