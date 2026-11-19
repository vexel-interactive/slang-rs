RWByteAddressBuffer data : register(u0);
RWStructuredBuffer<uint> output : register(u1);

static uint trace = 0;
RWByteAddressBuffer receiver() { trace = trace * 10 + 1; return data; }
uint address() { trace = trace * 10 + 2; return 12; }
uint inputValue() { trace = trace * 10 + 3; return 5; }
uint outputIndex() { trace = trace * 10 + 4; return 1; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    int signedOld = 0;
    uint old = 0;
    data.Store(0, 0xfffffffeu);
    data.InterlockedMin(0, -1, signedOld);
    output[0] = uint(signedOld);
    output[1] = data.Load(0);
    data.InterlockedMax(0, 3, old);
    output[2] = old;
    output[3] = data.Load(0);
    data.InterlockedAdd(0, 3, old);
    output[4] = old;
    output[5] = data.Load(0);
    data.InterlockedAnd(0, 6, old);
    output[6] = old;
    output[7] = data.Load(0);
    data.InterlockedOr(0, 9, old);
    output[8] = old;
    output[9] = data.Load(0);
    data.InterlockedXor(0, 3, old);
    output[10] = old;
    output[11] = data.Load(0);
    data.InterlockedExchange(0, 23, old);
    output[12] = old;
    output[13] = data.Load(0);
    data.InterlockedCompareStore(0, 23, 37);
    output[14] = data.Load(0);
    data.InterlockedCompareStore(0, 99, 41);
    output[15] = data.Load(0);
    data.InterlockedCompareExchange(0, 37, 41, old);
    output[16] = old;
    output[17] = data.Load(0);
    data.InterlockedCompareExchange(0, 37, 55, old);
    output[18] = old;
    output[19] = data.Load(0);

    data.Store(4, 10u);
    data.InterlockedMin(4, 7);
    data.InterlockedMax(4, 9);
    data.InterlockedAdd(4, 4);
    data.InterlockedAnd(4, 11);
    data.InterlockedOr(4, 4);
    data.InterlockedXor(4, 5);
    output[20] = data.Load(4);

    int2 components = int2(12, 13);
    data.Store(8, 0x80000001u);
    data.InterlockedMin(8, 4, components.y);
    output[21] = uint(components.x);
    output[22] = uint(components.y);
    output[23] = data.Load(8);
    int2x2 matrix = int2x2(1, 2, 3, 4);
    data.InterlockedMax(8, 0x80000002u, matrix[1][0]);
    output[24] = uint(matrix[1][0]);
    output[25] = uint(matrix[1][1]);
    output[26] = data.Load(8);

    data.Store(12, 7u);
    uint values[2] = {3, 4};
    receiver().InterlockedAdd(address(), inputValue(), values[outputIndex()]);
    output[27] = trace;
    output[28] = values[0];
    output[29] = values[1];
    output[30] = data.Load(12);

    data.Store(16, 0xffff0000u);
    uint position = 31;
    data.InterlockedAnd(16, 0xffff, output[position++]);
    output[32] = data.Load(16);
    output[33] = position;

    trace = 0;
    data.Store(12, 10u);
    receiver().InterlockedAdd(address(), inputValue(), old);
    output[34] = trace;
    output[35] = old;
    output[36] = data.Load(12);
    trace = 0;
    receiver().InterlockedAdd(address(), inputValue());
    output[37] = trace;
    output[38] = data.Load(12);
}
