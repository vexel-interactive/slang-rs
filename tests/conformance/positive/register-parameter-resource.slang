Texture2D<float> image : register(t3);
float read(Texture2D<float> input : register(s5)) : register(c2)
{
    return input[uint2(0)];
}
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = read(image); }
