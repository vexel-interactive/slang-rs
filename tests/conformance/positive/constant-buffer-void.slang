ConstantBuffer<void> input[2] : register(b0);
ConstantBuffer<float> value;
static uint count = 0;
uint nextIndex() { return count++; }
void read() { return input[nextIndex()]; }
void take(ConstantBuffer<void> handle) {}
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    read();
    take(input[nextIndex()]);
    output[0] = value + float(count);
}
