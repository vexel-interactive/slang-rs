Texture2D<float> image : register(t01) : register(t1, space00);
RWStructuredBuffer<float> output : register(u0);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = image[uint2(0)];
}
