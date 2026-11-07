RWStructuredBuffer<float4> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    bool2x3 flags = bool2x3(true);
    int3x2 integers = int3x2(1, 2, 3, 4, 5, 6);
    uint4x4 unsignedValues = uint4x4(7u);
    half2x2 halfValues = half2x2(1.5h);
    double4x3 doubleValues = double4x3(2.5);
    result[0] = (float4)flags._m00_m01_m10_m12;
    result[1] = (float4)integers._11_12_31_32;
    result[2] = (float4)unsignedValues._m00_m03_m30_m33;
    result[3] = (float4)halfValues._11_12_21_22;
    result[4] = (float4)doubleValues._m00_m02_m30_m32;
}
