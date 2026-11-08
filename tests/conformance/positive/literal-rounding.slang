RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = asuint(8388608.4999999999999999999f);
    output[1] = asuint(8388608.5f);
    output[2] = asuint(8388608.5000000000000000001f);
    output[3] = asuint(8388609.4999999999999999999f);
    output[4] = asuint(8388609.5f);
    output[5] = asuint(8388609.5000000000000000001f);
    output[6] = asuint(61456.hf);
    output[7] = asuint(61456.0001hf);
    output[8] = asuint(61488.hf);
}
