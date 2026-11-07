StructuredBuffer<uint> inputs;
RWStructuredBuffer<uint> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    bool input0 = (bool)inputs[0];
    uint count0 = countbits(input0);
    uint high0 = firstbithigh(input0);
    uint low0 = firstbitlow(input0);
    int reversed0 = reversebits(input0);
    results[0] = count0;
    results[1] = high0;
    results[2] = low0;
    results[3] = asuint(reversed0);
    bool1 input1 = (bool1)inputs[0];
    uint1 count1 = countbits(input1);
    uint1 high1 = firstbithigh(input1);
    uint1 low1 = firstbitlow(input1);
    int1 reversed1 = reversebits(input1);
    results[4] = count1[0];
    results[5] = high1[0];
    results[6] = low1[0];
    results[7] = asuint(reversed1[0]);
    bool2 input2 = (bool2)inputs[0];
    uint2 count2 = countbits(input2);
    uint2 high2 = firstbithigh(input2);
    uint2 low2 = firstbitlow(input2);
    int2 reversed2 = reversebits(input2);
    results[8] = count2[0];
    results[9] = high2[0];
    results[10] = low2[0];
    results[11] = asuint(reversed2[0]);
    bool3 input3 = (bool3)inputs[0];
    uint3 count3 = countbits(input3);
    uint3 high3 = firstbithigh(input3);
    uint3 low3 = firstbitlow(input3);
    int3 reversed3 = reversebits(input3);
    results[12] = count3[0];
    results[13] = high3[0];
    results[14] = low3[0];
    results[15] = asuint(reversed3[0]);
    bool4 input4 = (bool4)inputs[0];
    uint4 count4 = countbits(input4);
    uint4 high4 = firstbithigh(input4);
    uint4 low4 = firstbitlow(input4);
    int4 reversed4 = reversebits(input4);
    results[16] = count4[0];
    results[17] = high4[0];
    results[18] = low4[0];
    results[19] = asuint(reversed4[0]);
}
