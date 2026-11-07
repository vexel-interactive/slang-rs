struct Counter
{
    int value;
};
RWStructuredBuffer<Counter> counters;
RWTexture2D<uint> image;
groupshared int slots[2];

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    counters[0].value = 0;
    image[int2(0, 0)] = 0u;
    slots[0] = 0;
    int previous;
    uint old;
    InterlockedAdd(counters[0].value, 2.5, previous);
    InterlockedAdd(counters[0].value, 1);
    InterlockedAnd(counters[0].value, 3, previous);
    InterlockedAnd(counters[0].value, 3);
    InterlockedOr(counters[0].value, 4, previous);
    InterlockedOr(counters[0].value, 4);
    InterlockedXor(counters[0].value, 2, previous);
    InterlockedXor(counters[0].value, 2);
    InterlockedMin(counters[0].value, -1, previous);
    InterlockedMin(counters[0].value, -1);
    InterlockedMax(counters[0].value, 3, previous);
    InterlockedMax(counters[0].value, 3);
    InterlockedExchange(counters[0].value, 4, previous);
    InterlockedExchange(counters[0].value, 4);
    InterlockedCompareExchange(counters[0].value, 4, 5, previous);
    InterlockedCompareStore(counters[0].value, 5, 6);
    InterlockedAdd(image[int2(0, 0)], 1u, old);
    InterlockedAdd(image[int2(0, 0)], 1u);
    InterlockedAnd(image[int2(0, 0)], 3u, old);
    InterlockedAnd(image[int2(0, 0)], 3u);
    InterlockedOr(image[int2(0, 0)], 4u, old);
    InterlockedOr(image[int2(0, 0)], 4u);
    InterlockedXor(image[int2(0, 0)], 2u, old);
    InterlockedXor(image[int2(0, 0)], 2u);
    InterlockedMin(image[int2(0, 0)], 1u, old);
    InterlockedMin(image[int2(0, 0)], 1u);
    InterlockedMax(image[int2(0, 0)], 3u, old);
    InterlockedMax(image[int2(0, 0)], 3u);
    InterlockedExchange(image[int2(0, 0)], 4u, old);
    InterlockedExchange(image[int2(0, 0)], 4u);
    InterlockedCompareExchange(image[int2(0, 0)], 4u, 5u, old);
    InterlockedCompareStore(image[int2(0, 0)], 5u, 6u);
    InterlockedAdd(slots[0], 1, previous);
}
