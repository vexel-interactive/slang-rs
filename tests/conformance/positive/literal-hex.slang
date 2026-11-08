RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = 0x1.8p+1;
    output[1] = 0x1.ffcp15hf;
    output[2] = 0xffffffp104f;
    output[3] = 0x4p-151f;
    output[4] = float(0x4p-1076LF);
    output[5] = float(0x1.fffffffffffffp1023LF);
    output[6] = 0x1.000003p0;
    output[7] = 0x1.003p0hf;
    output[8] = 0x0p999999;
}
