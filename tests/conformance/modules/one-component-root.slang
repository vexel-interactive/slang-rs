import "one-component-types.slang";

struct Data
{
    Value value = { 2 };
    Matrix matrix = { { 3 } };
};

RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Data values[2] = { Data(), Data() };
    results[0] = values[0].value[0] + values[1].matrix[0][0];
}
