import library;
ConstantBuffer<Value> input;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    const int Count = 1;
    output[0] = input.z;
}
