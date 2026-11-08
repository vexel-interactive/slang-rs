RWStructuredBuffer<uint> output;
static int state = 0;
struct Pair { int first; int second; };

[ForceInline] Pair makePair(int x)
{
    if (x > 0) return Pair(x, x + 2);
    return Pair(-x, 7);
}
[ForceInline] Pair transform(Pair value, int addend)
{
    value.first += addend;
    if (addend > 0) value.second += addend;
    return value;
}
[ForceInline] int arrayValue(int values[2], bool change)
{
    if (change) values[0] += 3;
    int copy[2] = values;
    return copy[0] + copy[1];
}
[ForceInline] int[2] arrayResult(int x)
{
    int values[2] = { x, x + 1 };
    if (x < 0) { values[0] = -x; return values; }
    return values;
}
[ForceInline] void update(int x)
{
    if (x > 0) { state += x; return; }
    state += 7;
}
[ForceInline] void nestedVoid(int x) { return update(x); }
[ForceInline] void unusedInput(int x) {}
[ForceInline] int scopedArray(int x)
{
    int values[2] = { 3, 4 };
    { int values[2] = { x, x + 2 }; if (x > 2) return values[0] + values[1]; }
    return values[0] + values[1];
}
[ForceInline] int2x2 matrixResult(int x)
{
    if (x > 0) return int2x2(x, 2, 3, 4);
    return int2x2(5, 6, 7, 8);
}
[ForceInline] Pair localPair(int x) { Pair value = Pair(x, 3); value.first++; return value; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair value = makePair(3);
    output[0] = uint(value.first);
    output[1] = uint(value.second);
    Pair changed = transform(value, 4);
    output[2] = uint(changed.first + changed.second);
    output[3] = uint(value.first + value.second);
    output[4] = uint(makePair(-2).first + makePair(-2).second);
    int values[2] = { 5, 7 };
    output[5] = uint(arrayValue(values, true));
    output[6] = uint(values[0] + values[1]);
    int result[2] = arrayResult(-3);
    output[7] = uint(result[0] + result[1]);
    output[8] = uint(arrayResult(4)[1]);
    state = 1;
    update(3);
    output[9] = uint(state);
    nestedVoid(-1);
    output[10] = uint(state);
    unusedInput(state++);
    output[11] = uint(state);
    int comma = (update(2), state + 1);
    output[12] = uint(comma);
    output[13] = uint(state);
    (true ? update(1) : update(7));
    output[14] = uint(state);
    output[15] = uint(scopedArray(1));
    output[16] = uint(scopedArray(4));
    output[17] = uint(matrixResult(1)[0][1] + matrixResult(-1)[1][0]);
    output[18] = uint(localPair(4).first);
}
