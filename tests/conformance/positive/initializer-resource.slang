Texture2D<float> image;
SamplerState sampler;
RWByteAddressBuffer raw;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    raw.Store2({0u}, {1u, 2u});
    uint2 value = raw.Load2({0u});
    result[0] = image.SampleLevel(sampler, {0.5, 0.5}, {0.0})
        + dot({1.0, 2.0}, float2(3.0)) + float(value.x);
}
