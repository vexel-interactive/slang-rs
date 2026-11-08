typedef float Values[Count];
static const int Count = 3;
Values choose(bool flag);
Values choose(bool flag)
{
    if (flag) return {1, 2, 3};
    return Values(4, 5, 6);
}
float[3] copy(Values values) { return values; }
RWStructuredBuffer<float> result;

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Values values = copy(choose(true));
    result[0] = values[2] + choose(false)[1];
}
