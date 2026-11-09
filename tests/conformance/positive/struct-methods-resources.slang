struct Sample
{
    int value;
    [ForceInline] int read() { return value; }
    [ForceInline] [mutating] void store(int x) { value = x; }
    [ForceInline] [mutating] int[2] pair(int delta) { value += delta; return {value, value + 1}; }
};
ConstantBuffer<Sample> settings;
StructuredBuffer<Sample> input;
RWStructuredBuffer<Sample> items;
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint index = id.x;
    output[0] = uint(settings.read() + input[index].read());
    items[index++].store(3);
    output[1] = uint(items[index - 1].read());
    int pair[2] = items[index - 1].pair(4);
    output[2] = uint(pair[0] + pair[1]);
    output[3] = index;
}
