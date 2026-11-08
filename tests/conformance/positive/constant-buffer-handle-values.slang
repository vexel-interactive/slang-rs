ConstantBuffer<float> input[2];
RWStructuredBuffer<float> output;
float read(ConstantBuffer<float> value) { return value; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    output[0] = input[1] + read(input[0]);
}
