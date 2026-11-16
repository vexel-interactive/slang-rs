struct Tail
{
    uint64_t wide;
    float scalar;
};

struct Values
{
    Tail tail;
    float reusedTail;
    row_major float2x3 rows;
    column_major float2x3 columns;
    float values[2];
};

typedef float Pair[2];
typedef row_major float2x3 Rows;
typedef column_major float2x3 Columns;

ByteAddressBuffer input;
RWByteAddressBuffer output;

T read<T>(uint address)
{
    return input.Load<T>(address);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Values value = read<Values>(0);
    output.Store(0, value);
    Pair pair = input.Load<Pair>(80);
    output.Store<Pair>(80, pair);
    float scalar = input.LoadAligned<float>(88, 8);
    output.Store<float>(88, scalar, 8);
    output.Store(92, 1.5);
    output.Store(96, int(-2));
    output.Store(100, true);
    output.StoreAligned<float2>(104, float2(2.5, 3.5));
    output.Store2(112, uint2(7, 9));
    Rows rows = Rows(1, 2, 3, 4, 5, 6);
    output.Store(120, rows);
    Columns columns = Columns(7, 8, 9, 10, 11, 12);
    output.Store(144, columns);
    output.Store(168, rows + rows);
}
