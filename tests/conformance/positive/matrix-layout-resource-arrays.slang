struct Nested
{
    row_major float2x3 values[2][2];
};
struct Matrices
{
    row_major float2x3 rows[2];
    column_major float2x3 columns[2];
    Nested nested;
    uint tag;
};
StructuredBuffer<Matrices> input;
RWStructuredBuffer<Matrices> writable;
RWStructuredBuffer<float> output;
static uint cursor = 0;
uint nextIndex() { cursor++; return 0; }
float read(float2x3 values[2]) { return values[1][1][2]; }
void set(inout float value) { value = 2; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float2x3 values[2] = input[nextIndex()].rows;
    values = input.Load(0).columns;
    float2x3 nested[2][2] = input[0].nested.values;
    set(writable[nextIndex()].rows[0][0][0]);
    writable[0].columns[1][1][2] += read(input[0].rows);
    InterlockedAdd(writable[0].tag, 1u);
    output[0] = read(values) + read(nested[1]) + read(writable[0].rows) + cursor;
}
