static const int count = 1;
static float value = 2.0;

float read(float value)
{
    {
        // The initializer reads the parameter before introducing the new local.
        float value = value + 1.0, next = value + 1.0;
        return next;
    }
}

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    // These initializers read globals; later references read the new locals.
    const int count = count + 1;
    float value = value + 1.0;
    switch (count)
    {
        case count:
            result[0] = read(value);
            break;
        default:
            result[0] = 0.0;
            break;
    }
}
