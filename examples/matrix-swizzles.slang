RWStructuredBuffer<float4> result;

void writePair(out float2 pair)
{
    pair = float2(7, 8);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 matrixValue = float2x3(1, 2, 3, 4, 5, 6);
    matrixValue._m00_m12 = float2(9, 10);
    matrixValue._12_21 += float2(1, 2);
    writePair(matrixValue._m02_m11);
    result[0] = matrixValue._m00_m01_m10_m12;
    result[1] = matrixValue._11_11_23_23;
}
