typedef row_major float2x3 Row;
struct Parameters
{
    Row matrix : packoffset(c2);
    column_major float2x3 column;
    float tail;
};
ConstantBuffer<Parameters> parameters;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    output[0] = parameters.matrix[1][2] + parameters.column[0][1] + parameters.tail;
}
