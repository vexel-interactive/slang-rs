struct Parameters
{
    float unused[0];
    float scale : packoffset(c1.x);
};
ConstantBuffer<Parameters> unused[0];
ConstantBuffer<Parameters> input;
cbuffer Direct
{
    float offset;
    float alsoUnused[0];
    float tail;
};
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = input.scale + offset + tail;
}
