typedef Texture2D<float4> Textures[2];
typedef SamplerState Samplers[2];
typedef ByteAddressBuffer Bytes[2];
typedef StructuredBuffer<float> Buffers[2];
Textures copy(Textures values) { return values; }
Samplers copy(Samplers values) { return values; }
Bytes copy(Bytes values) { return values; }
Buffers copy(Buffers values) { return values; }
Textures images;
Samplers samplers;
Bytes bytes;
Buffers buffers;
RWStructuredBuffer<float4> result;
Textures fallback(Textures values = copy(images)) { return values; }

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    result[0] = fallback()[1].SampleLevel(copy(samplers)[0], float2(0), 0)
        + float(copy(bytes)[0].Load(0)) + copy(buffers)[1][0];
}
