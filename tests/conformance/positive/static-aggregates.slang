static int seed = 3;
struct Packet
{
    int value = seed;
};

int run(int input)
{
    static int value = seed + input, next = value + 1;
    static int values[2] = {value, next};
    static Packet packet = Packet();
    for (static int i = 0, j = 2; i < 3; i++)
        next += j;
    return values[0] + values[1] + packet.value + next;
}

RWStructuredBuffer<int> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int value = run(1);
    result[0] = value + run(5);
}
