Texture2D<float> first[2];
Texture2D<float> fixed : register(t1);
Texture2D<float> second;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first[1][uint2(0)] + fixed[uint2(0)] + second[uint2(0)];
}
