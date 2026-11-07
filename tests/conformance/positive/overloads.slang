RWStructuredBuffer<float> result;

float twice(float value)
{
    return value * 2.0;
}

int twice(int value)
{
    return value * 2;
}

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = twice(float(id.x)) + float(twice(int(id.x)));
}
