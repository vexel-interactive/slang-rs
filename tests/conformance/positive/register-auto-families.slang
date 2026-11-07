struct Params
{
    float scale;
};

ConstantBuffer<Params> params;
Texture2D<float4> image;
StructuredBuffer<float> input;
ByteAddressBuffer bytes;
SamplerState sampler;
RWTexture2D<float> writable;
RWByteAddressBuffer rwBytes;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float value = image.SampleLevel(sampler, float2(0), 0).x;
    value += input[0] + float(bytes.Load(0)) + params.scale;
    writable[uint2(0)] = value;
    rwBytes.Store(0, uint(value));
    output[0] = writable[uint2(0)] + float(rwBytes.Load(0));
}
