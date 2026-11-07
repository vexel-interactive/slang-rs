Texture1D<float> line;
Texture3D<float4> volume;
RWTexture2D<float4> output;
StructuredBuffer<float4> values;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint count;
    uint stride;
    values.GetDimensions(count, stride);
    uint width;
    uint height;
    output.GetDimensions(width, height);
    float4 loaded = volume.Load(int4(0, 0, 0, 0));
    float4 indexed = volume[uint3(0, 0, 0)];
    float lineValue = line.Load(int2(0, 0));
    output[id.xy] = loaded + indexed + values.Load(0) + lineValue;
    output[id.xy].x += float(count + stride + width + height);
}
