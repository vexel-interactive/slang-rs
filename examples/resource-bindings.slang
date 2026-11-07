struct Parameters
{
    float scale;
};

ConstantBuffer<Parameters> parameters : register(b0, space1);
Texture2D<float4> images[2] : register(t0, space1);
SamplerState imageSampler : register(s0, space1);
RWStructuredBuffer<float4> output : register(u0, space1);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = images[1].SampleLevel(imageSampler, float2(0), 0) * parameters.scale;
}
