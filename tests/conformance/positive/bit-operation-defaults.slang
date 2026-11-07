static uint mask = 15u;
RWStructuredBuffer<uint> results;

uint read(uint value = countbits(mask)) { return value; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint mask = 1u;
    results[0] = read();
}
