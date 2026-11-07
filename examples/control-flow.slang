RWStructuredBuffer<float> result;

float accumulate(uint count)
{
    float value = 0.0;
    for (uint i = 0; i < count; i++)
    {
        if (i == 2)
        {
            continue;
        }
        value += float(i);
    }
    return value;
}

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = accumulate(id.x);
}
