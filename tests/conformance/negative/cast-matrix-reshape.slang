RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float3x2 value = (float3x2)float2x3(1.0);
    result[0] = value[0][0];
}
