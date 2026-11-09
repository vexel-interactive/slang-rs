struct Ops
{
    T copy<T>(T value) { return value; }
    T unused<T>(T value) { return T(); }
    int unused(StructuredBuffer<int> value) { return 1; }
};
struct Item { int value; };
ConstantBuffer<Item> uniformInput;
StructuredBuffer<int> input;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Ops ops = {};
    output[0] = uint(ops.copy(uniformInput).value);
    output[1] = uint(ops.copy(input)[0]);
    Item item = {3};
    output[2] = uint(ops.copy<Item>(item).value);
    output[3] = uint(ops.unused(input));
}
