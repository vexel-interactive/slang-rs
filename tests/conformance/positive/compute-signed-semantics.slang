RWStructuredBuffer<int4> output;
[shader("compute")][numthreads(4, 1, 1)]
void main(int3 dispatch : SV_DispatchThreadID, int3 group : SV_GroupID,
    int3 local : SV_GroupThreadID, int index : SV_GroupIndex)
{
    output[dispatch.x] = int4(dispatch.x, group.x, local.x, index);
}
