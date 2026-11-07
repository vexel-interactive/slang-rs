import "../modules/auto-resources";

Texture2D<float> image;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = image[uint2(0)] + imported[uint2(0)];
}
