RWByteAddressBuffer bytes : register(u0);
RWStructuredBuffer<uint> output : register(u1);

static uint trace = 0;

RWByteAddressBuffer receiver()
{
    bytes.Store(0, 7u);
    trace = trace * 10 + 1;
    return bytes;
}

uint address()
{
    trace = trace * 10 + 2;
    return 0u;
}

uint inputValue()
{
    trace = trace * 10 + 3;
    return 5u;
}

typedef row_major float2x3 RowMat;
typedef RowMat MatPair[2];

// Capturing this initializer must preserve its row-major matrix-array value.
static MatPair startup = (
    receiver().InterlockedAdd(address(), inputValue()),
    MatPair(RowMat(1, 2, 3, 4, 5, 6), RowMat(7, 8, 9, 10, 11, 12)));

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(startup[0][1][0]);
    output[1] = uint(startup[1][0][2]);
    output[2] = trace;
    output[3] = bytes.Load(0);
}
