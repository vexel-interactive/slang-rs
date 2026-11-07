RWStructuredBuffer<float> result;

float choose(bool flag)
{
    float value = 1.0;
    if (flag)
    {
        float value = 2.0;
        return value;
    }
    else
    {
        return value;
    }
}

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = choose(id.x > 0);
}
