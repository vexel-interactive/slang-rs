StructuredBuffer<float2x3> matrices;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    matrices[0]._m00 = 1;
}
