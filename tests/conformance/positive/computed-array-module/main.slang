import library;
ConstantBuffer<Values> input;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const int Count = 2;
    float values[Count] = float[Count](1, 2);
    result[0] = values[1] + input[2];
}
