float run(float vector, float matrix)
{
    return (vector < 3.0 && matrix > 1.0) ? 1.0 : 2.0;
}

RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = run(float(id.x), 2.0);
}
