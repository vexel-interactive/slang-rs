groupshared uint counter;
RWStructuredBuffer<uint> result;

[shader("compute")]
[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID, uint groupIndex : SV_GroupIndex)
{
    if (groupIndex == 0)
        counter = 0;
    GroupMemoryBarrierWithGroupSync();

    uint previous;
    InterlockedAdd(counter, 1u, previous);
    GroupMemoryBarrierWithGroupSync();
    result[id.x] = counter;
}
