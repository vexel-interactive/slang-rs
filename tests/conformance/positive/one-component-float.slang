RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float1 vector = float1(2);
    results[0] = float(vector[0]);
    float1x1 value1 = float1x1(2);
    results[1] = float(value1[0][0]);
    float1x2 value2 = float1x2(2, 2);
    results[2] = float(value2[0][1]);
    float1x3 value3 = float1x3(2, 2, 2);
    results[3] = float(value3[0][2]);
    float1x4 value4 = float1x4(2, 2, 2, 2);
    results[4] = float(value4[0][3]);
    float2x1 value5 = float2x1(2, 2);
    results[5] = float(value5[1][0]);
    float3x1 value6 = float3x1(2, 2, 2);
    results[6] = float(value6[2][0]);
    float4x1 value7 = float4x1(2, 2, 2, 2);
    results[7] = float(value7[3][0]);
}
