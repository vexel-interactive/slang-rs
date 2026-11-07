RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(4, 1, 2)]
void main(uint index : SV_GroupIndex)
{
    result[index] = fwidth_fine(float(index));
}
