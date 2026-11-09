interface Make { __init(int value = 3); int read(); }
struct Item : Make
{
    int value;
    __init(int input = 9) { value = input + 10; }
    __init(float input) { value = 20; }
    int read() { return value; }
}
int make<T : Make>() { T item = T(1.0); return item.read(); }
int makeDefault<T : Make>() { T item = T(); return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    output[0] = make<Item>();
    output[1] = makeDefault<Item>();
    Item item = Item();
    output[2] = item.read();
}
