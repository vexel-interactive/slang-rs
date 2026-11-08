typedef float Values[3];
ConstantBuffer<Values> input;
Values copy(Values values) { return values; }
Values load(ConstantBuffer<Values> buffer) { return buffer; }
RWStructuredBuffer<float> result;

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Values values = copy(input);
    result[0] = values[2] + load(input)[1];
}
