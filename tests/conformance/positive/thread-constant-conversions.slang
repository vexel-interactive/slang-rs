static const uint width = 0xffffffffu + 9u;
static const int height = uint(2);
RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(width, height, true)]
void main(uint index : SV_GroupIndex)
{
    results[index] = width;
}
