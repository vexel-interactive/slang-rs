RWStructuredBuffer<uint> output;
static int a = 99;
struct Value
{
    int a = 2;
    int b = 4;
    [ForceInline] __init(int x) { a = x; }
    [ForceInline] __init(float x, int bias = 3) { this.a = int(x) + bias; this.b = 7; }
    [ForceInline] __init(int x, out int result) { int a = x + 2; this.a = a; b = x + 3; result = x + 4; }
}
struct Early
{
    int value;
    [ForceInline] __init(int x) { if (x > 0) { value = x; return; } this.value = 8; }
    [ForceInline] __init(float x) { return Early(int(x) + 2); }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Value first = Value(9);
    output[0] = uint(first.a);
    output[1] = uint(first.b);
    output[2] = uint(a);
    Value second = Value(9.0);
    output[3] = uint(second.a);
    output[4] = uint(second.b);
    Value third = { 5.0, 6 };
    output[5] = uint(third.a);
    output[6] = uint(third.b);
    Value copied = Value(first);
    output[7] = uint(copied.a);
    int result;
    Value fourth = Value(10, result);
    output[8] = uint(fourth.a);
    output[9] = uint(fourth.b);
    output[10] = uint(result);
    Early positive = Early(5);
    Early negative = Early(-1);
    Early converted = Early(4.0);
    output[11] = uint(positive.value);
    output[12] = uint(negative.value);
    output[13] = uint(converted.value);
    int index = 0;
    Value evaluated = Value(index++);
    output[14] = uint(evaluated.a);
    output[15] = uint(index);
}
