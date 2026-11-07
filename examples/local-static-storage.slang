RWStructuredBuffer<int> result;
static int initializationCount;

int initialize(int seed)
{
    initializationCount++;
    return seed;
}

int accumulate(int amount)
{
    static int total = initialize(amount);
    total += amount;
    return total;
}

int delayed(bool enter)
{
    if (enter)
        static int value = initialize(10);
    return value;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int total = delayed(false);
    total += delayed(true);
    total += delayed(true);
    total += accumulate(2);
    total += accumulate(3);
    result[0] = total + initializationCount;
}
