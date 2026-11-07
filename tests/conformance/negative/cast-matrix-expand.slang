RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float3x3 value = (float3x3)float2x2(1.0);
    result[0] = value[0][0];
}
