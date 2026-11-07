struct Pair
{
    float value = 3.0;
};

static Pair global = Pair(2.0);
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = global.value;
}
