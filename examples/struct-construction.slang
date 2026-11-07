struct Pair
{
    float left = 2.0;
    float right = 3.0;
};

struct Container
{
    Pair values[2];
    float scale = 4.0;
};

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair pair = Pair(5.0);
    Pair copy = Pair(pair);
    Container container = Container();
    result[0] = copy.left + copy.right + container.values[1].left * container.scale;
}
