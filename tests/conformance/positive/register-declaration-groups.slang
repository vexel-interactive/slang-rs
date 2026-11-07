#define IMAGE_SLOT t01
#define IMAGE_SPACE space02

Texture2D<float> first : register(IMAGE_SLOT.x, IMAGE_SPACE),
                 second : register(t3, space0);
RWStructuredBuffer<float> output : register(u0);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first[uint2(0)] + second[uint2(0)];
}
