RWStructuredBuffer<int16_t2> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = (int16_t2) float3(1);
}
