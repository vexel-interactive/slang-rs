typedef Texture2D<float> Image;
typealias Outputs = RWStructuredBuffer<float>[2];

Image image : register(t4, space3);
Outputs outputs : register(u6, space3);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    outputs[1][0] = image[uint2(0)];
}
