RWByteAddressBuffer data : register(u0);
RWStructuredBuffer<uint> output : register(u1);

static uint trace = 0;

uint seed()
{
    data.Store(0, 7u);
    return 0u;
}

static uint seeded = seed();

RWByteAddressBuffer receiver()
{
    trace = trace * 10 + 1;
    return data;
}

uint address()
{
    trace = trace * 10 + 2;
    return 0u;
}

uint inputValue()
{
    trace = trace * 10 + 3;
    return 5u;
}

// The preceding seed must run first, then these arguments in source order.
// Slang 2026.21 drops this atomic side effect from its static comma initializer.
static uint startup = (receiver().InterlockedAdd(address(), inputValue()), 0u);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = trace;
    output[1] = data.Load(0);
    output[2] = seeded;
    output[3] = startup;
}
