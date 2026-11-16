interface IValue
{
    func value() -> int;
}

interface IMarker
{
    func marker() -> int;
}

struct Item : IValue, IMarker
{
    func value() -> int
    {
        return 3;
    }

    func marker() -> int
    {
        return 5;
    }
}

func sum<T>(input : T) -> int where T : IValue, IMarker
{
    return input.value() + input.marker();
}

func pair<T, U>(first : T, second : U) -> int where T : IValue where U : IMarker
{
    return first.value() + second.marker();
}

func intersection<T>(input : T) -> int where T : IValue & IMarker
{
    return input.value() + input.marker();
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
func main()
{
    Item value;
    output[0] = uint(sum(value));
    output[1] = uint(pair<Item, Item>(value, value));
    output[2] = uint(intersection(value));
}
