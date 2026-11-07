RWStructuredBuffer<float4> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 matrixValue = float2x3(1);
    result[0] = float4(matrixValue._m00_12);
}
