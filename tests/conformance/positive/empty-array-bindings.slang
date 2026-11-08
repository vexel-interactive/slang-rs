struct Empty
{
    float values[0];
    Texture2D<float4> textures[0];
};
Empty value;
ConstantBuffer<Empty> empty;
Texture2D<float4> textures[0] : register(t1);
SamplerState samplers[0];
ConstantBuffer<float> buffers[65536][65536][0];
ConstantBuffer<float> input;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = input;
}
