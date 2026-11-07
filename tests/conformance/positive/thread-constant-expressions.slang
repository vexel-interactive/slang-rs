RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(1 << 3, 0x2u, (int)1.0)]
void main(uint index : SV_GroupIndex)
{
    results[index] = index;
}
