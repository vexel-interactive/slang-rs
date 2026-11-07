RWTexture3D<float4> image;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    image[uint2(0, 0)] = float4(1.0);
}
