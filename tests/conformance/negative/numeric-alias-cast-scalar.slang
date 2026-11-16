RWStructuredBuffer<uint32_t> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = (uint32_t) float2(1);
}
