int run(int seed)
{
    int total = 0;
    for (static int i = seed; i < 4; i++)
        total += i;
    return total;
}

RWStructuredBuffer<int> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int total = run(2);
    result[0] = total + run(1);
}
