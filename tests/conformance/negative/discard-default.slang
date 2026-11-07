float read()
{
    discard;
    return 1.0;
}

float sample(float value = read())
{
    return value;
}

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = sample();
}
