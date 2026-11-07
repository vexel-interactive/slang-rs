static const int width = 4;
RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(width, 1, 1)]
void main(uint width : SV_GroupIndex)
{
    results[width] = width;
}
