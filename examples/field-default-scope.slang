static float baseValue = 2.0;

float readBase()
{
    return baseValue;
}

struct Pair
{
    float x = baseValue;
    float values[2] = {readBase(), baseValue + 1.0};
};

struct Container
{
    Pair pair;
};

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float baseValue = 9.0;
    float readBase = 8.0;
    Pair named = Pair();
    Pair listed = {};
    Pair supplied = Pair(baseValue);
    Container nested = {};
    Pair array[2] = Pair[2]();

    // Omitted fields refer to globals; supplied arguments use this local scope.
    result[0] = named.x + listed.values[1] + supplied.x + nested.pair.x
        + array[1].values[0] + readBase;
}
