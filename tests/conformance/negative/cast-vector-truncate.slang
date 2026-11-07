RWStructuredBuffer<float2> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = (float2)float3(1.0);
}
