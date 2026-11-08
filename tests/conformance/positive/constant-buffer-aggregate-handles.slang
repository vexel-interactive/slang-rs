struct Parameters
{
    float3 values[2];
    row_major float2x3 matrix;
};
typedef Parameters Values[2];
ConstantBuffer<Values> input[2];
static uint cursor = 0;
uint nextIndex() { cursor++; return 1; }
ConstantBuffer<Values> handle(uint index) { return input[index]; }
float read(Values values = input[0])
{
    return values[1].values[1].z + values[0].matrix[1][2];
}
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    Values copy = handle(nextIndex());
    output[0] = read(handle(nextIndex())) + read() + copy[0].values[1].z + cursor;
}
