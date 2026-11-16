T zero<T>()
{
    return T();
}

int dependency<T>()
{
    T ignored = zero<T>();
    return 23;
}

interface IBuild
{
    __init(int input);
}

struct Value<T> : IBuild
{
    T marker;
    int value;

    __init(float input, int extra = dependency<int>())
    {
        value = int(input) + extra;
    }
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = 0;
}
