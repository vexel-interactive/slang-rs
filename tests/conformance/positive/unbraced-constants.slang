int classify(bool enter)
{
    if (enter)
        const int label = 1 + 1;
    switch (2)
    {
        case label:
            return 3;
        default:
            return 0;
    }
}

float read(bool enter)
{
    if (enter)
        const float value = 2.5;
    return value;
}

RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = read(false) + float(classify(false));
}
