struct Parameters
{
    float2x3 implicit;
    column_major float2x3 column;
    row_major float2x3 row;
    float tail;
};
ConstantBuffer<Parameters> parameters;
float2x3 data;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    output[0] = parameters.implicit[0][1] + parameters.column[1][2]
        + parameters.row[0][2] + parameters.tail + data[1][2];
}
