RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    bool1 vector = bool1(true);
    results[0] = float(vector[0]);
    bool1x1 value1 = bool1x1(true);
    results[1] = float(value1[0][0]);
    bool1x2 value2 = bool1x2(true, true);
    results[2] = float(value2[0][1]);
    bool1x3 value3 = bool1x3(true, true, true);
    results[3] = float(value3[0][2]);
    bool1x4 value4 = bool1x4(true, true, true, true);
    results[4] = float(value4[0][3]);
    bool2x1 value5 = bool2x1(true, true);
    results[5] = float(value5[1][0]);
    bool3x1 value6 = bool3x1(true, true, true);
    results[6] = float(value6[2][0]);
    bool4x1 value7 = bool4x1(true, true, true, true);
    results[7] = float(value7[3][0]);
}
