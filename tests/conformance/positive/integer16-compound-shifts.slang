RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int16_t x = int16_t(1);
    x <<= 16;
    output[0] = uint(x);
    uint16_t y = uint16_t(65535);
    y >>= 16;
    output[1] = uint(y);
    int16_t z = int16_t(-32768);
    z >>= 16;
    output[2] = uint(z);

    int16_t values[2] = { int16_t(1), int16_t(3) };
    int i = 0;
    int16_t value = (values[i++] <<= 16);
    output[3] = uint(value);
    output[4] = uint(values[0]);
    output[5] = uint(i);
}
