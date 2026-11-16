interface IBuild
{
    __init(int input);
}

struct Value : IBuild
{
    int value;

    __init(float input, int extra = 2)
    {
        value = int(input) + extra;
    }

    __init(float input, int extra = 2, int other = 3)
    {
        value = int(input) + extra + other;
    }
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
}
