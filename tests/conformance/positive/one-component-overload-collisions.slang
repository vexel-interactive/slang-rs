int choose(float value[2]) { return 1; }
int choose(float1 value[2]) { return 2; }
int choose__overload_1() { return 3; }
RWStructuredBuffer<int> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int choose__overload_0 = 4;
    float1 values[2] = { 1, 2 };
    results[0] = choose(values) + choose__overload_0 + choose__overload_1();
}
