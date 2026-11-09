struct Item { int value; };
struct Marker
{
    int value;
    __init<T>(T ignored) { value = 7; }
};
struct Handle<T>
{
    T handle;
    __init<U>(U ignored, T input) { handle = input; }
};
struct Dormant
{
    int value;
    __init<T>(T input) { T empty = T(); value = 13; }
    __init(StructuredBuffer<int> input) { value = 17; }
};
StructuredBuffer<int> input;
ConstantBuffer<Item> constants;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(Marker(input).value);
    output[1] = uint(Marker(constants).value);
    Handle<StructuredBuffer<int>> buffer = Handle<StructuredBuffer<int>>(3.0, input);
    output[2] = uint(buffer.handle[0]);
    Handle<ConstantBuffer<Item>> constant = Handle<ConstantBuffer<Item>>(int2(1, 2), constants);
    output[3] = uint(constant.handle.value);
    output[4] = uint(Dormant(input).value);
}
