import library;
ConstantBuffer<Values> input;
RWStructuredBuffer<float> result;

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Values values = copy(input);
    result[0] = values[2] + load(input)[1];
}
