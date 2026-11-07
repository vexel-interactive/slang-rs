Texture2D<float> first;
Texture2D<float> fixed : register(t1);
Texture2D<float> second;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first[uint2(0)] + fixed[uint2(0)] + second[uint2(0)];
}
