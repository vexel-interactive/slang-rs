RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    // Integer casts retain signedness and the destination width.
    output[0] = uint((int16_t) 65535);
    output[1] = uint((uint16_t) 65537);
    output[2] = uint((int32_t) 0xffffffffu);
    output[3] = (uint32_t) -1;
    output[4] = uint((int64_t) -1);
    output[5] = uint((uint64_t) 0x100000001UL);
    output[6] = uint((intptr_t) -2);
    output[7] = uint((uintptr_t) 0x100000003UL);
    output[8] = uint((size_t) 4);
    output[9] = uint((usize_t) 5);
    output[10] = uint((ssize_t) -6);

    // Floating aliases preserve the sign of zero during conversion.
    output[11] = asuint((float16_t) -0.0f);
    output[12] = asuint((float32_t) -0.0lf);
    uint low;
    uint high;
    asuint((float64_t) -0.0f, low, high);
    output[13] = low;
    output[14] = high;

    int16_t2 pair = (int16_t2) int2(65535, 65538);
    uint32_t3 triple = (uint32_t3) int3(-1, 2, 3);
    float64_t2x2 matrixValue = (float64_t2x2) float3x3(1, 2, 3, 4, 5, 6, 7, 8, 9);
    output[15] = uint(pair.x);
    output[16] = uint(pair.y);
    output[17] = triple.x;
    output[18] = triple.z;
    output[19] = uint(matrixValue[1][1]);

    uint index = 0;
    int16_t once = (int16_t) index++;
    output[20] = uint(once);
    output[21] = index;
}
