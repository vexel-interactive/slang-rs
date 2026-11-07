RWStructuredBuffer<float4> result;

[shader("compute")]
[numthreads(2, 2, 1)]
void main(uint3 thread : SV_GroupThreadID, uint index : SV_GroupIndex)
{
    float2 value = float2(thread.x, thread.y);
    float2 coarse = fwidth_coarse(value);
    float2 fine = fwidth_fine(value);
    result[index] = float4(coarse + fine, ddx(value.x), ddy(value.y));
}
