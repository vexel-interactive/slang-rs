RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(2, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    result[index] = ddx(float(index));
}
