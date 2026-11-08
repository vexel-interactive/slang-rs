typedef row_major float2x3 Row;
typealias Rows = Row[2];
typedef column_major float2x3 Column;
struct Parameters
{
    Row row;
    Column column;
    row_major Column overridden;
    Rows values;
    float tail;
};
ConstantBuffer<Parameters> parameters;
row_major float2x3 globals[2];
RWStructuredBuffer<float> output;

Row copy(Row value)
{
    row_major column_major float2x3 local = (row_major float2x3)value;
    return (column_major matrix<float,2,3>)local;
}
float read(float2x3 values[2])
{
    return values[1][1][2];
}
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float2x3 values[2] = parameters.values;
    output[0] = copy(parameters.row)[0][1] + parameters.column[1][2]
        + parameters.overridden[0][2] + parameters.tail + globals[1][1][2]
        + read(parameters.values) + read(values);
}
