RWStructuredBuffer<uint> output;
[ForceInline] int add(int x) { int y = x + 2; return y; }
[ForceInline] int choose(int x) { if (x > 0) return x + 3; return 7; }
[ForceInline] int[2] makeArray(int x) { int value[2] = { x, x + 1 }; return value; }
static int scalar = add(9);
static int branch = choose(-1);
static int values[2] = makeArray(3);
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(scalar);
    output[1] = uint(branch);
    output[2] = uint(values[0] + values[1]);
}
