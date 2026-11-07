void reject(float value)
{
    discard;
}

void reject(int value)
{
}

float read()
{
    discard;
    return 1.0;
}

float sample(float value = read())
{
    return value;
}

struct Pair
{
    float value = read();
};

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    reject(1);
    Pair pair = Pair(3.0);
    result[0] = pair.value + sample(2.0);
}
