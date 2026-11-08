// Values follow the pinned frontend literal corpus; stores expose the
// converted IEEE bits to the DXIL constant comparison.
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = asuint(0x1p-126f);
    output[1] = asuint(0x1p-149f);
    output[2] = asuint(0x1.fffffep127f);
    output[3] = asuint(0x1p-14hf);
    output[4] = asuint(0x1p-24hf);
    output[5] = asuint(0x1.ffcp15hf);
    output[6] = asuint(-0.0hf);
    output[7] = asuint(-0.0f);
    output[8] = asuint(0#INFf);
    output[9] = asuint(-1#INFhf);
    uint low, high;
    asuint(0x1p-1022LF, low, high);
    output[10] = low;
    output[11] = high;
    asuint(0x1p-1074LF, low, high);
    output[12] = low;
    output[13] = high;
    asuint(0x1.fffffffffffffp1023LF, low, high);
    output[14] = low;
    output[15] = high;
    asuint(-0.0LF, low, high);
    output[16] = low;
    output[17] = high;
    asuint(1.0000000000000002lf, low, high);
    output[18] = low;
    output[19] = high;
}
