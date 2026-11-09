struct Box<T>
{
    T value;
    T read() { return value; }
};
struct Item { int value; };
StructuredBuffer<int> input;
ConstantBuffer<Box<Item>> settings;
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Box<StructuredBuffer<int>> buffer = {input};
    Box<Item> aggregate = {{3}};
    output[0] = uint(buffer.read()[0] + aggregate.read().value + settings.value.value);
}
