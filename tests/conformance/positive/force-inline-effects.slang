StructuredBuffer<int> input;
StructuredBuffer<uint> tables[2];
struct Params { int value; };
ConstantBuffer<Params> buffers[2];
RWStructuredBuffer<uint> output;
RWByteAddressBuffer bytes;
static int counter = 0;
groupshared uint sharedValue;

[ForceInline] int step(int x) { if (x < 0) return -x; return x + 1; }
void storeValue(inout uint destination, int value) { destination = uint(value); }
int readHandle(ConstantBuffer<Params> value, int addend) { return value.value + addend; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID)
{
    int limit = input[tid.x];
    int i = 0, sum = 0;
    while (step(i) < limit) { i++; if (i == 2) continue; sum += i; }
    do { i++; if (i == 4) continue; sum += i; } while (step(i) < limit);
    for (int j = step(0); step(j) < limit; j = step(j))
    {
        switch (j) { case 2: continue; default: sum += j; break; }
    }
    storeValue(output[counter++], step(sum));
    uint width, stride;
    input.GetDimensions(width, stride);
    output[step(0)] = width + stride;
    uint old;
    InterlockedAdd(sharedValue, uint(step(sum)), old);
    bytes.InterlockedAdd(uint(step(0)) * 4u, uint(step(sum)), old);
    output[step(1)] = old;
    uint2 dimensions;
    input.GetDimensions(dimensions[step(-1)], dimensions[step(0) - 1]);
    output[step(2)] = dimensions.x + dimensions.y;
    tables[counter].GetDimensions(dimensions[step(counter++) - 1], dimensions[1]);
    output[step(3)] = dimensions.x + dimensions.y;
    counter = 0;
    output[step(4)] = uint(readHandle(buffers[counter++], step(sum)));
}
