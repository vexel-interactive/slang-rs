interface IBuild
{
    __init(int input);
    __init(uint input);
}

struct Value : IBuild
{
    int value;

    __init(float input, int extra = 3)
    {
        value = int(input) + extra;
    }
}

T fromSigned<T : IBuild>(int input)
{
    return T(input);
}

T fromUnsigned<T : IBuild>(uint input)
{
    return T(input);
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(fromSigned<Value>(2).value);
    output[1] = uint(fromUnsigned<Value>(4u).value);
    output[2] = uint(fromSigned<Value>(5).value);
    output[3] = uint(Value(2).value);
}
