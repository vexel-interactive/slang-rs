float matrix(float value)
{
    return value + 1.0;
}

float run(float vector)
{
    return matrix(vector);
}

RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = run(2.0);
}
