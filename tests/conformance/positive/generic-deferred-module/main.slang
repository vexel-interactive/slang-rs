import lib;
internal struct Item { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Item item = {7};
    output[0] = uint(relay(item).value);
    output[1] = uint(create<Item>().value);
}
