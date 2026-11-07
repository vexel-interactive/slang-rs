import "../modules/bound-resources";

RWStructuredBuffer<float> output : register(u4, space5);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = readImage();
}
