Texture1D<float> line;
Texture2D<float4> image;
Texture3D<float2> volume;
TextureCube<float3> cube;
SamplerState imageSampler;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float a = line.SampleGrad(imageSampler, 0.5, 0.01, 0.01);
    float4 b = image.SampleGrad(imageSampler, {0.5, 0.5}, {0.01, 0.0}, {0.0, 0.01});
    float2 c = volume.SampleGrad(imageSampler, float3(0.5), float3(0.01), float3(0.01));
    float3 d = cube.SampleGrad(imageSampler, float3(1.0), float3(0.01), float3(0.01));
    result[0] = a + b.x + c.x + d.x;
}
