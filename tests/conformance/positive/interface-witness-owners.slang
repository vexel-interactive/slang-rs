interface I<Value> { int read(int input); Value copy(Value input); }
struct Item<Value> : I<Value>
{
    int read(int input) { return 4; }
    int read(float input) { return 9; }
    Value copy(Value input) { return input; }
}
struct Box<T : I<int>>
{
    T item;
    struct Runner
    {
        int run<U : I<float>>(U other) { return other.read(1.0); }
    }
    int read() { return item.read(1.0); }
}
int read<Value, T : I<Value>>(T item, Value input)
{
    item.copy(input);
    return item.read(1.0);
}
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item<int> item;
    Item<float> other;
    Box<Item<int>> box = {item};
    Box<Item<int>>::Runner runner;
    output[0] = box.read();
    output[1] = runner.run(other);
    output[2] = read(item, 3);
    output[3] = read(other, 5.0);
}
