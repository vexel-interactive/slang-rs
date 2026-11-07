typedef int Count;
static const int width = height + 2;
static const Count height = depth + 1;
static const int depth = { Count(1) };
RWStructuredBuffer<int> results;

[shader("compute")]
[numthreads(width, height, depth)]
void main(uint index : SV_GroupIndex)
{
    results[index] = width + height + depth;
}
