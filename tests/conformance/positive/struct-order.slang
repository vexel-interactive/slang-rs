struct Pair
{
    float z;
    int a;
};

RWStructuredBuffer<float> result;

float next()
{
    return 2.0;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair value = Pair(next(), 3);
    result[0] = value.z + float(value.a);
}
