RWStructuredBuffer<uint> output;
static int3 vectorState = int3(1, 2, 3);
struct Pair { int first; int second; };

[ForceInline] int aliased(inout int a, inout int b) { a += 2; return b; }
[ForceInline] int readAfterArgument(inout int a, int b) { return a + b; }
[ForceInline] int useVector(inout int2 value, int addend) { value.x += addend; return value.x + value.y; }
[ForceInline] int changeVector() { vectorState.x = 7; return 3; }
[ForceInline] int overlapping(inout int2 a, inout int2 b) { a.x = 9; b.y = 8; return a.x + b.y; }
[ForceInline] void initialize(out int value, int input) { if (input > 0) { value = input * 2; return; } value = 7; }
[ForceInline] void increment(inout int value) { value += 2; }
[ForceInline] void shiftView(inout uint value) { value >>= 1; }
[ForceInline] uint mixedView(inout uint a, inout int b) { a = 9u; return uint(b); }
[ForceInline] uint vectorView(inout uint2 value, inout int2 other) { value.x = 9u; value.y >>= 1; return uint(other.x) + uint(other.y); }
[ForceInline] uint increments(inout uint value) { uint old = value++; uint now = ++value; return old + now; }
[ForceInline] void incrementUnsigned(inout uint value) { value += 2u; }
[ForceInline] void nestedView(inout uint value) { incrementUnsigned(value); }
[ForceInline] void writeUnsigned(out uint value) { value = 0xffffffffu; }
void ordinaryUnsigned(inout uint value) { value += 3u; }
[ForceInline] void viewCall(inout uint value) { ordinaryUnsigned(value); }
[ForceInline] void splitValue(double value, out uint lo, out uint hi) { asuint(value, lo, hi); }
[ForceInline] Pair writePair(inout Pair value) { value.first += 3; return value; }
[ForceInline] void writeArray(out int values[2], int x) { values[0] = x; values[1] = x + 1; }
[ForceInline] int initializerShadow(int x) { { int x = x + 2; return x; } }
[ForceInline] int indexedOutput(inout int value, inout int coordinate) { coordinate = 0; value += 4; return value; }
[ForceInline] uint matrixView(inout uint2x2 value, inout int2x2 other) { value._m00 = 7u; return uint(other[0][0]); }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int value = 1;
    output[0] = uint(aliased(value, value));
    output[1] = uint(value);
    value = 1;
    output[2] = uint(readAfterArgument(value, (value = 5)));
    output[3] = uint(value);
    output[4] = uint(useVector(vectorState.xy, changeVector()));
    output[5] = uint(vectorState.x);
    int3 v = int3(1, 2, 3);
    output[6] = uint(overlapping(v.xy, v.yz));
    output[7] = uint(v.x * 100 + v.y * 10 + v.z);
    v.x = 1;
    output[8] = uint(aliased(v.x, v.x));
    int2x2 m = int2x2(1, 2, 3, 4);
    output[9] = uint(aliased(m._m00, m[0][0]));
    int values[2] = { 3, 7 };
    int index = 0;
    output[10] = uint(readAfterArgument(values[index++], (index = 1, 3)));
    output[11] = uint(index);
    initialize(value, 4);
    output[12] = uint(value);
    initialize(value, -1);
    output[13] = uint(value);
    int converted = -2;
    shiftView(converted);
    output[14] = uint(converted);
    Pair pair = Pair(2, 5);
    Pair result = writePair(pair);
    output[15] = uint(pair.first + result.second);
    writeArray(values, 4);
    output[16] = uint(values[0] + values[1]);
    output[17] = uint(initializerShadow(3));
    value = 1;
    output[18] = mixedView(value, value);
    output[19] = uint(value);
    int2 vectorValue = int2(1, -2);
    output[20] = vectorView(vectorValue, vectorValue);
    output[21] = uint(vectorValue.x);
    output[22] = uint(vectorValue.y);
    value = -1;
    output[23] = increments(value);
    output[24] = uint(value);
    v = int3(1, 2, 3);
    output[25] = uint(aliased(v.xy[0], v.x));
    output[26] = uint(aliased(m._m00_m11[1], m[1][1]));
    value = -1;
    nestedView(value);
    output[27] = uint(value);
    writeUnsigned(value);
    output[28] = uint(value);
    value = 1;
    viewCall(value);
    output[29] = uint(value);
    int lo, hi;
    splitValue(1.0, lo, hi);
    output[30] = uint(lo);
    output[31] = uint(hi);
    v = int3(1, 2, 3);
    index = 1;
    output[32] = uint(indexedOutput(v[index++], index));
    output[33] = uint(v.x * 100 + v.y * 10 + v.z);
    output[34] = uint(index);
    output[35] = uint(aliased(v.yx.xy.y, v.x));
    index = 1;
    output[36] = uint(indexedOutput(m[index++][1], index));
    output[37] = uint(m[1][1]);
    output[38] = matrixView(m, m);
    output[39] = uint(m[0][0]);
    output[40] = uint(aliased(v.yx[1u], v.x));
}
