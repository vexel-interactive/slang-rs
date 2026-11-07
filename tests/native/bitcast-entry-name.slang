StructuredBuffer<float> inputs;
RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void asfloat()
{
    results[0] = asuint(inputs[0]);
}
