RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float3 value = float3(1, 2, 3);
    result[0] = (float)value;
}
