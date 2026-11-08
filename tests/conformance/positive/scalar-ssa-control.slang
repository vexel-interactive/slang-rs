StructuredBuffer<uint> input;
RWStructuredBuffer<uint> output;

int merged(bool flag)
{
    int x = 1;
    if (flag) x = 7;
    else x = 7;
    return x + 2;
}

int invariant(int count)
{
    int i = 0, x = 7;
    while (i < count) { i++; x = 7; }
    return x;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(merged(input[0] != 0));
    int x = 3;
    if (false) x = int(input[0]);
    x <<= 32;
    output[1] = uint(x);
    output[2] = uint(invariant(int(input[0])));

    int one = 1;
    while (false) one = 3;
    do { one += 4; } while (false);
    output[3] = uint(one);

    int switched = 1;
    for (int i = 0; i < 4; i++)
    {
        switch (i) { case 0: switched += 2; break; default: continue; }
        break;
    }
    switch (1)
    {
    case 0: switched = 9; break;
    case 1: switched += 3;
    case 2: switched += 4; break;
    default: switched = 99;
    }
    output[4] = uint(switched);

    int shortValue = 1;
    bool a = false && ((shortValue = 9) > 0);
    bool b = true || ((shortValue = 8) > 0);
    int selected = true ? (shortValue += 2) : (shortValue += 4);
    output[5] = uint(shortValue + selected + int(a) + int(b));

    int compound = 1;
    compound += (compound = 3);
    compound += compound++;
    output[6] = uint(compound);

    int sequenced = 1;
    int comma = (sequenced += 2, sequenced <<= 32, 4);
    output[7] = uint(comma);
    output[8] = uint(sequenced);

    uint unsignedValue = 0xffffffffu;
    unsignedValue >>= 32;
    output[9] = unsignedValue;
    int signedValue = -2;
    signedValue >>= 32;
    output[10] = uint(signedValue);
    int negativeCount = 1;
    negativeCount <<= -1;
    output[11] = uint(negativeCount);

    uint64_t wide = 0xffffffffffffffffULL;
    wide /= 3ULL;
    output[12] = uint(wide);
    output[13] = uint(wide >> 32);
    intptr_t pointer = intptr_t(-1);
    pointer >>= 64;
    output[14] = uint(pointer);
}
