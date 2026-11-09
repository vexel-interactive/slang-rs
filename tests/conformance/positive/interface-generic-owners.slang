interface I { int read(); }
struct Item : I { int value; int read() { return value; } }
struct Box<T : I> { T item; int read() { return item.read(); } }
struct Runner { int run<T : I>(T item) { return item.read(); } }
struct Result { int value; __init<T : I>(T item) { value = item.read(); } }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item = {4};
    Box<Item> box = {item};
    Runner runner;
    Result result = Result(item);
    output[0] = box.read();
    output[1] = runner.run(item);
    output[2] = result.value;
}
