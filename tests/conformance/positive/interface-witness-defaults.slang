int seed<T>() { return 3; }
interface I { int read(int input = seed<int>()); }
struct Item : I { int read(int input = 9) { return input; } }
int read<T : I>(T item) { int seed = 20; return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item item;
    output[0] = read(item);
    output[1] = read<Item>(item);
    output[2] = item.read();
}
