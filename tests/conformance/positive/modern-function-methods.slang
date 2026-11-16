interface IValue
{
    func read(input : int = 2) -> int;
    [mutating] func update(input : int);
}

struct Box<T> : IValue
{
    T marker;
    int value;

    func read(input : int = 2) -> int
    {
        return value + input;
    }

    [mutating] func update(input : int)
    {
        value = input;
    }

    static func offset(input : int) -> int
    {
        return input + 1;
    }

    func copy<U>(input : U) -> U
    {
        return input;
    }
}

func invoke<T : IValue>(value : T) -> int
{
    return value.read();
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
func main()
{
    Box<float> value = { 0, 3 };
    output[0] = uint(invoke(value));
    value.update(7);
    output[1] = uint(value.read(1));
    output[2] = uint(Box<float>::offset(3));
    output[3] = uint(value.copy<int>(6));
}
