struct Params
{
    float scale;
};

ConstantBuffer<Params> params : register(b2, space3);
Texture2D<float4> image : register(t4, space1);
TextureCube<float4> cube : register(t5, space1);
StructuredBuffer<float> input : register(t6);
ByteAddressBuffer bytes : register(t8, space2);
RWByteAddressBuffer rwBytes : register(u9, space2);
SamplerState sampler : register(s10);
SamplerComparisonState comparison : register(s11, space4);
Texture2D<float> depth : register(t12);
RWTexture3D<float> writable : register(u5, space1);
RWStructuredBuffer<float> output : register(u7);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float value = image.SampleLevel(sampler, float2(0), 0).x;
    value += cube.SampleLevel(sampler, float3(0, 0, 1), 0).x;
    value += depth.SampleCmpLevelZero(comparison, float2(0), 0);
    value += input[0] + float(bytes.Load(0)) + params.scale;
    rwBytes.Store(0, uint(value));
    writable[uint3(0)] = value;
    output[0] = writable[uint3(0)] + float(rwBytes.Load(0));
}
