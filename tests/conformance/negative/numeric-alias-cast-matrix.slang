RWStructuredBuffer<float64_t> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float64_t3x3 value = (float64_t3x3) float2x2(1);
    output[0] = value[0][0];
}
