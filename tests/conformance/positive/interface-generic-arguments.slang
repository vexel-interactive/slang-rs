interface Read<Value> { Value get(); }
struct Item : Read<int> { int value; int get() { return value; } }
U read<U, T : Read<U>>(T item, U ignored) { return item.get(); }
struct Box<T : Read<int>> { T item; int get() { return item.get(); } }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item = {5};
    Box<Item> box = {item};
    output[0] = read(item, 0);
    output[1] = read<int, Item>(item, 0);
    output[2] = box.get();
}
