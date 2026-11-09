interface I { int read(int input); }
struct Item : I
{
    int read(int input) { return 1; }
    int read(float input) { return 2; }
    static int read(int input, int extra = 0) { return 3; }
}
int read<T : I>(T item) { return item.read(1.0); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item item;
    output[0] = read(item);
    output[1] = read<Item>(item);
    output[2] = item.read(1.0);
}
