typedef float3 Values[2];
ConstantBuffer<Values> input;
RWStructuredBuffer<float> output;
float read(float3 values[2]) { return values[0].x + values[1].y; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    Values values = input;
    Values constructed = Values(input);
    output[0] = input[1].z + read(input) + read(values) + read(constructed);
}
