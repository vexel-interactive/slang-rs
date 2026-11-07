import "thread-sizes.slang";

RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(width, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    results[index] = width;
}
