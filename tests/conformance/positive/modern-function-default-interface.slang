interface IValue
{
    func read(input : int) -> int;

    func run() -> int
    {
        return read(2);
    }
}

struct Item : IValue
{
    func read(input : int) -> int
    {
        return input + 3;
    }
}

func invoke<T : IValue>(input : T) -> int
{
    return input.run();
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
func main()
{
    Item value;
    output[0] = uint(invoke(value));
}
