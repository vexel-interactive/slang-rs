typedef row_major float2x3 Row;
typedef column_major float2x3 Column;
typealias RowBuffer = StructuredBuffer<Row>;
RowBuffer rows;
StructuredBuffer<column_major float2x3> columns;
RWStructuredBuffer<Row> writable;
RWStructuredBuffer<float> output;
static uint cursor = 0;
uint nextIndex() { cursor++; return 0; }
RowBuffer getRows() { return rows; }
float read(StructuredBuffer<Row> values) { return values[0][1][2]; }
float read(StructuredBuffer<Column> values) { return values.Load(0)[0][1]; }
void set(inout float value) { value = 2; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    Row value = getRows()[nextIndex()];
    writable[0] = rows.Load(0);
    set(writable[0][1][2]);
    output[0] = value[0][1] + read(rows) + read(columns) + writable[0][1][2] + cursor;
}
