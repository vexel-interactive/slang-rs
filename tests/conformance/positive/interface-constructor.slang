interface Make { __init(int value); void read(out int value); }
struct Item : Make
{
    int value;
    __init(int input) { value = input; }
    void read(out int output) { output = value; }
}
int read<T : Make>(int input)
{
    T item = T(input);
    int output;
    item.read(output);
    return output;
}
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { output[0] = read<Item>(7); }
