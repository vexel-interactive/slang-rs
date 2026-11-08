struct Params { int value; };
ConstantBuffer<Params> buffers[2];
StructuredBuffer<int> input[2];
RWStructuredBuffer<int> output;
Texture2D<float4> texture;
SamplerState sampler;
RWByteAddressBuffer bytes;

[ForceInline] int loadBuffer(StructuredBuffer<int> value, int index)
{
    if (index < 0) return value[0];
    return value[index];
}
[ForceInline] StructuredBuffer<int> selected(bool flag, StructuredBuffer<int> a, StructuredBuffer<int> b)
{
    if (flag) return a;
    return b;
}
[ForceInline] ConstantBuffer<Params> selectedUniform(bool flag, ConstantBuffer<Params> a, ConstantBuffer<Params> b)
{
    if (flag) return a;
    return b;
}
[ForceInline] int readUniform(ConstantBuffer<Params> value) { return value.value; }
[ForceInline] float4 sampleValue(Texture2D<float4> value, SamplerState state, float2 uv)
{
    if (uv.x < 0) return value.Load(int3(0, 0, 0));
    return value.SampleLevel(state, uv, 0);
}
[ForceInline] void storeBytes(RWByteAddressBuffer value, uint offset, int x)
{
    if (x < 0) { value.Store(offset, uint(-x)); return; }
    value.Store(offset, uint(x));
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID)
{
    bool flag = tid.x == 0;
    StructuredBuffer<int> chosen = selected(flag, input[0], input[1]);
    output[0] = loadBuffer(chosen, int(tid.x));
    output[1] = readUniform(selectedUniform(true, buffers[0], buffers[1]));
    output[2] = int(sampleValue(texture, sampler, float2(float(tid.x), 0)).x);
    storeBytes(bytes, tid.x * 4u, loadBuffer(input[1], int(tid.x)));
}
