RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    double1 vector = double1(2);
    results[0] = float(vector[0]);
    double1x1 value1 = double1x1(2);
    results[1] = float(value1[0][0]);
    double1x2 value2 = double1x2(2, 2);
    results[2] = float(value2[0][1]);
    double1x3 value3 = double1x3(2, 2, 2);
    results[3] = float(value3[0][2]);
    double1x4 value4 = double1x4(2, 2, 2, 2);
    results[4] = float(value4[0][3]);
    double2x1 value5 = double2x1(2, 2);
    results[5] = float(value5[1][0]);
    double3x1 value6 = double3x1(2, 2, 2);
    results[6] = float(value6[2][0]);
    double4x1 value7 = double4x1(2, 2, 2, 2);
    results[7] = float(value7[3][0]);
}
