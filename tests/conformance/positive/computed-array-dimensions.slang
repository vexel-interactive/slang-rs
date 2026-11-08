typedef float Row[Count];
static const int Count = Next + 1;
static const int Next = 2;
ConstantBuffer<Row> input;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const int Count = 2;
    float[Count + 1] a = float[3.5](1, 2, 3);
    float b[true][0x3] = {a};
    float c[3 > 2 ? 3 : 1] = float[6 >> 1](4, 5, 6);
    result[0] = b[0][2] + c[2] + input[2];
}
