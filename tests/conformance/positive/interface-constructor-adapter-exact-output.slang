interface IBuild
{
    __init(int input);
}

struct Value : IBuild
{
    int value;

    __init(out int input)
    {
        input = 4;
        value = 5;
    }

    __init(float input)
    {
        value = int(input) + 20;
    }
}

T make<T : IBuild>(int input)
{
    return T(input);
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int input = 3;
    Value adapted = make<Value>(input);
    output[0] = uint(adapted.value);
    output[1] = uint(input);
    output[2] = uint(Value(3.0).value);
}
