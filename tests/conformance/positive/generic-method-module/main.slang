import lib;
internal struct Item { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Ops ops = {};
    Item value = {9};
    output[0] = uint(ops.relay(value).value);
    output[1] = uint(ops.create<Item>().value);
}
