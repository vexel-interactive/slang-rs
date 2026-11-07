StructuredBuffer<uint> inputs;
RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    uint bits = inputs[index];
    int signedBits = asint(bits);
    results[0] = countbits(bits);
    results[1] = reversebits(bits);
    results[2] = firstbithigh(bits);
    results[3] = firstbithigh(signedBits);
    results[4] = firstbitlow(signedBits);

    uint3 values = uint3(bits, 0u, 0xffffffffu);
    uint3 counts = countbits(values);
    results[5] = counts.y + counts.z;
}
