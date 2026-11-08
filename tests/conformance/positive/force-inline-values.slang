RWStructuredBuffer<uint> output;

[ForceInline] uint16_t narrowIdentity(uint16_t x) { return x; }
[ForceInline] int16_t narrowAdd(int16_t x, int16_t y) { return x + y; }
[ForceInline] int16_t narrowChoose(bool pick, int16_t a, int16_t b) { return pick ? a : b; }
uint16_t ordinaryIdentity(uint16_t x) { return x; }

[ForceInline] int pair(int a, int b) { return a * 10 + b; }
[ForceInline] int ignored(int a, int b) { return a; }
[ForceInline] int modified(int x) { return (x += 2) + x; }
[ForceInline] int locals(int seed)
{
    int x = seed + 2, y = x + 3;
    x += y;
    return x;
}
[ForceInline] int nested(int x) { return pair(x, x); }
[ForceInline] bool positive(int x) { return x > 0; }
[ForceInline] float2 splat(float x) { return float2(x, x + 1.0); }
[ForceInline] float2x2 matrixValue(float x) { return float2x2(x, x + 1.0, x + 2.0, x + 3.0); }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint16_t x = narrowIdentity(uint16_t(1)); x <<= 16;
    output[0] = uint(x);
    int16_t y = narrowAdd(int16_t(32767), int16_t(1)); y >>= 16;
    output[1] = uint(y);
    int16_t z = narrowChoose(true, int16_t(3), int16_t(4)); z <<= 16;
    output[2] = uint(z);
    uint16_t ordinary = ordinaryIdentity(uint16_t(1)); ordinary <<= 16;
    output[3] = uint(ordinary);

    int i = 1;
    output[4] = uint(pair(i++, i++));
    output[5] = uint(i);
    output[6] = uint(ignored(i++, i++));
    output[7] = uint(i);
    int original = 10;
    output[8] = uint(modified(original));
    output[9] = uint(original);
    output[10] = uint(locals(1));
    output[11] = uint(nested(2));
    int selected = false ? pair(i++, 99) : ignored(7, i++);
    output[12] = uint(selected);
    output[13] = uint(i);
    bool shortValue = false && positive(i++);
    output[14] = uint(shortValue);
    output[15] = uint(i);
    float2 vectorValue = splat(float(i++));
    output[16] = uint(vectorValue.x);
    output[17] = uint(vectorValue.y);
    output[18] = uint(i);
    float2x2 matrix = matrixValue(1.0);
    output[19] = uint(matrix[1][1]);
}
