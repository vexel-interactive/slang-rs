struct Params
{
    float scale;
};

Texture2D<float4> images[2][3] : register(t2, space4);
ConstantBuffer<Params> params[2] : register(b3);
RWStructuredBuffer<float> outputs[2] : register(u0);
ByteAddressBuffer bytes[2] : register(t0, space1);
RWByteAddressBuffer rwBytes[2] : register(u2);
SamplerState samplers[2] : register(s5);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float value = images[1][2].SampleLevel(samplers[1], float2(0), 0).x;
    value += params[1].scale + float(bytes[1].Load(0));
    rwBytes[1].Store(0, uint(value));
    outputs[1][0] = value + float(rwBytes[1].Load(0));
}
