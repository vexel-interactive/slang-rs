static float accumulator = 0.0;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    accumulator = float(id.x);
    result[id.x] = accumulator;
}
