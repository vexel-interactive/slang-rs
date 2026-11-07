RWStructuredBuffer<uint> result;
static uint initialized = (InterlockedExchange(result[0], 2u), 1u);
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[1] = initialized;
}
