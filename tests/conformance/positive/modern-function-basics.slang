func add(first : int, int second = 3) -> int;

int add(int first, int second)
{
    return first + second;
}

func copy<T = int>(value : T) -> T
{
    return (T) value;
}

func assign(out target : int, value : int)
{
    target = value;
}

func pair(first : int) -> int[2]
{
    return int[2](first, first + 1);
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
func main(id : uint3 : SV_DispatchThreadID,)
{
    int value = 0;
    assign(value, add(4));
    output[0] = uint(value);
    output[1] = uint(copy<>(5));
    output[2] = uint(copy(6));
    int values[2] = pair(8);
    output[3] = uint(values[0] + values[1]);
}
