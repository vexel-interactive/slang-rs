struct Params { int value; };
ConstantBuffer<Params> buffers[2];
RWStructuredBuffer<int> output;

[ForceInline] ConstantBuffer<Params> selected(bool flag, ConstantBuffer<Params> a, ConstantBuffer<Params> b)
{
    if (flag) return a;
    return b;
}
[ForceInline] int readValue(ConstantBuffer<Params> value) { return value.value; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID)
{
    output[0] = readValue(selected(tid.x == 0, buffers[0], buffers[1]));
}
