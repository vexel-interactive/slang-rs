struct Data
{
    static const int Count = 3;
    static int bias = 4;
    int value;
};
ConstantBuffer<Data> settings;
StructuredBuffer<Data> input;
RWStructuredBuffer<Data> values;
RWStructuredBuffer<uint> output;
struct Access
{
    static ConstantBuffer<Data> constants = settings;
    static StructuredBuffer<Data> source = input;
    static RWStructuredBuffer<Data> target = values;
    static groupshared uint sharedValue;
    [ForceInline] static void write(uint id)
    {
        target[id].value = source[id].value + constants.value + Data.bias;
        sharedValue = uint(target[id].value);
    }
};
[shader("compute")]
[numthreads(Data.Count, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    Access.write(id.x);
    output[id.x] = Access.sharedValue + uint(settings.bias + values[id.x].bias);
}
