#define SCALE(x) ((x) * 2.0)
#define ENABLED 1

#if defined(ENABLED) && ENABLED == 1
RWStructuredBuffer<float> result;
#else
#error wrong branch
#endif

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = SCALE(float(id.x));
}
