RWStructuredBuffer<uint> output;
static int state = 0;
static int cells[2] = { 3, 4 };

[ForceInline] int choose(int a, int b)
{
    if (a > b) return a * 2;
    return b + 3;
}
[ForceInline] int scoped(int x)
{
    int value = x;
    { int value = x + 7; if (value > 10) return value; }
    if (x < 0) { int value = 91; return value; }
    return value + 2;
}
[ForceInline] int forReturn(int limit)
{
    int sum = 0;
    for (int i = 0; i < limit; i++)
    {
        if (i == 1) continue;
        sum += i + 1;
        if (i == 3) return sum;
        if (i == 6) break;
    }
    return sum + 10;
}
[ForceInline] int whileReturn(int limit)
{
    int i = 0;
    while (i < limit) { i++; if (i == 2) continue; if (i == 4) return i + 20; }
    return i + 30;
}
[ForceInline] int doReturn(int limit)
{
    int i = 0;
    do { i++; if (i == 2) continue; if (i == 4) return i + 40; } while (i < limit);
    return i + 50;
}
[ForceInline] int switched(int x)
{
    int sum = 1;
    switch (x)
    {
    case 0: sum += 3;
    case 1: sum += 4; break;
    case 2: return 17;
    default: sum = 19;
    }
    return sum + 2;
}
[ForceInline] int change(int x) { if (x > 0) { state += x; return state; } state += 10; return state; }
[ForceInline] int next(int x) { if (x >= 0) return x + 1; return 0; }
[ForceInline] bool isLess(int x, int limit) { if (x < limit) return true; return false; }
[ForceInline] int countStatic() { static int value = 7; value++; return value; }
[ForceInline] int nestedFlow(int x) { if (x > 0) return choose(next(x), 2); return scoped(x); }
[ForceInline] int writeCell(int x) { if (x > 0) { cells[0] = 7; return 2; } return 0; }
[ForceInline] int cellIndex() { if (state >= 0) { cells[0] = 8; return 0; } return 1; }
[ForceInline] int returnBeforeIncrement()
{
    for (int i = 0; isLess(i, 8); state = change(2)) { return 9; }
    return 3;
}
[ForceInline] int switchContinue(int count)
{
    int sum = 0;
    for (int i = 0; i < count; i++)
    {
        switch (i) { case 1: continue; case 3: return sum; default: sum += i; break; }
        sum++;
    }
    return sum;
}
int pair(int a, int b) { return a * 10 + b; }
int setInput(inout int target, int x) { target += x; return target; }
int readCells(int values[2], int x) { return values[0] + values[1] + x; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(choose(5, 2));
    output[1] = uint(choose(1, 4));
    output[2] = uint(scoped(2));
    output[3] = uint(scoped(6));
    output[4] = uint(scoped(-1));
    output[5] = uint(forReturn(0));
    output[6] = uint(forReturn(3));
    output[7] = uint(forReturn(8));
    output[8] = uint(whileReturn(3));
    output[9] = uint(whileReturn(9));
    output[10] = uint(doReturn(0));
    output[11] = uint(doReturn(3));
    output[12] = uint(doReturn(9));
    output[13] = uint(switched(0));
    output[14] = uint(switched(1));
    output[15] = uint(switched(2));
    output[16] = uint(switched(9));
    state = 1;
    output[17] = uint(state + change(2));
    output[18] = uint(state);
    state = 1;
    output[19] = uint(pair(state++, change(3)));
    output[20] = uint(state);
    state = 1;
    output[21] = uint(false && change(3) > 0);
    output[22] = uint(true || change(3) > 0);
    output[23] = uint(state);
    output[24] = uint(true ? change(2) : change(7));
    output[25] = uint(state);
    output[26] = uint((state += 2, change(1)));
    state = 1;
    state += change(2);
    output[27] = uint(state);
    int values[2] = { 3, 7 };
    int index = 0;
    values[index++] += next(index);
    output[28] = uint(values[0]);
    output[29] = uint(index);
    index = 0;
    int old = setInput(values[index++], next(index));
    output[30] = uint(old);
    output[31] = uint(index);
    int i = 0, sum = 0;
    for (int ignored = 0; isLess(i, 4); i = next(i)) { if (i == 1) continue; sum += i; }
    output[32] = uint(sum);
    i = 0; sum = 0;
    do { i++; if (i == 2) continue; sum += i; } while (isLess(i, 3));
    output[33] = uint(sum);
    i = 0; sum = 0;
    while (isLess(i, 4)) { i++; if (i == 2) continue; sum += i; }
    output[34] = uint(sum);
    output[35] = uint(countStatic());
    output[36] = uint(countStatic());
    output[37] = uint(nestedFlow(4));
    output[38] = uint(nestedFlow(-1));
    state = 0;
    cells[state++] += writeCell(1);
    output[39] = uint(cells[0]);
    output[40] = uint(state);
    cells[0] = 3;
    output[41] = uint(cells[cellIndex()]);
    int2 vectorValue = int2(4, 7);
    index = 0;
    vectorValue[index++] = next(index);
    output[42] = uint(vectorValue.x);
    int2x2 matrixValue = int2x2(1, 2, 3, 4);
    state = 0;
    matrixValue[state++][change(1) - 1] += next(4);
    output[43] = uint(matrixValue[0][1]);
    output[44] = uint(state);
    state = 1;
    output[45] = uint(returnBeforeIncrement());
    output[46] = uint(state);
    output[47] = uint(switchContinue(7));
    cells[0] = 3;
    cells[1] = 4;
    output[48] = uint(readCells(cells, writeCell(1)));
}
