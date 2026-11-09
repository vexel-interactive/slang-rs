interface IValue
{
    int read();
    static int base();
    [mutating] void set(int value);
    void copyOut(out int value);
}

struct Item : IValue
{
    int value;
    int read() { return value; }
    static int base() { return 2; }
    [mutating] void set(int input) { value = input; }
    void copyOut(out int output) { output = value; }
}

int read<T : IValue>(T item) { return item.read() + T::base(); }
void set<T : IValue>(inout T item, int input) { item.set(input); }
int copy<T : IValue>(T item)
{
    int output;
    item.copyOut(output);
    return output;
}

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item = {3};
    output[0] = read(item);
    set(item, 7);
    output[1] = read<Item>(item);
    output[2] = copy(item);
}
