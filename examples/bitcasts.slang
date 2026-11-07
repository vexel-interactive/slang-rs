StructuredBuffer<float> inputs;
RWStructuredBuffer<uint> words;
RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    float input = inputs[index];
    uint bits = asuint(input);
    results[index] = asfloat(bits);
    words[index] = asuint(asint(input));

    double wide = double(input);
    uint low;
    int high;
    asuint(wide, low, high);
    double restored = asdouble(low, high);
    results[index + 1] = float(restored);

    uint2 lowWords = uint2(low, 0u);
    uint2 highWords = uint2(uint(high), 0x3ff00000u);
    double2 pair = asdouble(lowWords, highWords);
    results[index + 2] = float(pair.y);
}
