Texture2D<float> image : register(t2147483648);
RWStructuredBuffer<float> output : register(u0);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = image[uint2(0)];
}
