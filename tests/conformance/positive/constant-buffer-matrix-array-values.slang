typedef row_major float2x3 Matrices[2];
ConstantBuffer<Matrices> input[2] : register(b3,space2);
RWStructuredBuffer<float> output;
static uint cursor = 0;
uint nextIndex() { cursor++; return 1; }
float read(float2x3 values[2]) { return values[1][1][2]; }
float readHandle(ConstantBuffer<Matrices> buffer) { return read(buffer); }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float2x3 values[2] = input[nextIndex()];
    output[0] = read(input[0]) + readHandle(input[1]) + values[0][0][1]
        + input[1][1][1][2] + cursor;
}
