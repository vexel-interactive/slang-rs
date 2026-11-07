RWStructuredBuffer<float2x3> matrices;
RWStructuredBuffer<int> result;
groupshared int2x3 counters;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2 integral;
    float2 fractional = modf(float2(1.25, 2.5), matrices[0]._m00_m12);
    matrices[0]._12_21 = fractional;
    counters = int2x3(0);
    int previous;
    InterlockedAdd(counters._m00, 1, previous);
    result[0] = previous + counters._11;
}
