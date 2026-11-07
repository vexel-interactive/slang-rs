Texture2D<float> first[3] : register(t2, space1);
Texture2D<float> second : register(t4, space1);
RWStructuredBuffer<float> output : register(u0);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first[2][uint2(0)] + second[uint2(0)];
}
