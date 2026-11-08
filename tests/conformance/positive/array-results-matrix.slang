typedef row_major float2x3 Matrices[2];
Matrices make();
Matrices make()
{
    return Matrices(float2x3(1, 2, 3, 4, 5, 6), float2x3(7, 8, 9, 10, 11, 12));
}
RWStructuredBuffer<float> result;

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Matrices values = make();
    result[0] = values[1][1][2];
}
