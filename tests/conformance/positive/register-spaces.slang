Texture2D<float> first : register(t0, space1);
Texture2D<float> second : register(t0, space2);
Texture2D<float> third : register(t0, space0);
RWStructuredBuffer<float> output : register(u0, space1);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first[uint2(0)] + second[uint2(0)] + third[uint2(0)];
}
