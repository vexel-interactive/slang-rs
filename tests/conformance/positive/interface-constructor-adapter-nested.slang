interface IBuild<T>
{
    __init(T input);
}

struct Outer<T>
{
    typealias Marker = T;

    struct Value : IBuild<int>
    {
        Marker marker;
        int value;

        __init(float input, int extra = 2)
        {
            value = int(input) + extra;
        }
    }
}

T make<T : IBuild<int>>(int input)
{
    return T(input);
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(make<Outer<float>.Value>(3).value);
    output[1] = uint(make<Outer<float>.Value>(4).value);
}
