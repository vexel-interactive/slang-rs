static const int width = height * 2;
static const int height = 2;
RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(width, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    results[index] = ddx(float(index));
}
