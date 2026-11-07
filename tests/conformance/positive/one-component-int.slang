RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int1 vector = int1(2);
    results[0] = float(vector[0]);
    int1x1 value1 = int1x1(2);
    results[1] = float(value1[0][0]);
    int1x2 value2 = int1x2(2, 2);
    results[2] = float(value2[0][1]);
    int1x3 value3 = int1x3(2, 2, 2);
    results[3] = float(value3[0][2]);
    int1x4 value4 = int1x4(2, 2, 2, 2);
    results[4] = float(value4[0][3]);
    int2x1 value5 = int2x1(2, 2);
    results[5] = float(value5[1][0]);
    int3x1 value6 = int3x1(2, 2, 2);
    results[6] = float(value6[2][0]);
    int4x1 value7 = int4x1(2, 2, 2, 2);
    results[7] = float(value7[3][0]);
}
