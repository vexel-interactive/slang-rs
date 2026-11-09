interface Make { __init(int input = 3); int read(); }
struct Item : Make
{
    int value;
    __init(int input = 9) { value = input + 10; }
    __init(float input) { value = 20; }
    int read() { return value; }
}
struct Box<T : Make>
{
    T value;
    __init() {}
    int read() { return value.read(); }
}
int lists<T : Make>()
{
    T item = {1.0};
    T array[2] = {{2.0}, {3.0}};
    return item.read() + array[0].read() + array[1].read();
}
int automatic<T : Make>() { T item; return item.read(); }
T result<T : Make>() { return {1.0}; }
int defaultInput<T : Make>(T item = {2.0}) { return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    output[0] = lists<Item>();
    output[1] = automatic<Item>();
    Box<Item> box;
    output[2] = box.read();
    output[3] = result<Item>().read();
    output[4] = defaultInput<Item>();
}
