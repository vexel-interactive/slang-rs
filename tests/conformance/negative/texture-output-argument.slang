Texture2D<float4> image;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint height;
    image.GetDimensions(1u, height);
}
