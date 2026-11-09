struct Box<T>
{
    T value;
    __init(T input) { value = input; }
    T read() { return value; }
    [mutating] void set(T input) { value = input; }
    Box<T> copy() { return this; }
};
struct Pair<T = int, U = T> { T first; U second; };
typealias Wrapped<T> = Box<T>;
typedef int32_t Integer;
int read(Box<int> input) { return input.value; }
struct Counter<T>
{
    static int count = 0;
    static int next() { count++; return count; }
};
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Box<int> first = Box<int>(3);
    Box<float> second = Box<float>(4);
    first.set(7);
    output[0] = uint(first.copy().read());
    output[1] = uint(second.read());
    Wrapped<Integer> alias = Wrapped<Integer>(9);
    output[2] = uint(read(alias));
    Pair<> defaults = Pair<>(2, 5);
    Pair<int, float> mixed = {3, 6};
    output[3] = uint(defaults.first + defaults.second);
    output[4] = uint(mixed.first + mixed.second);
    output[5] = uint(Counter<int>.next());
    output[6] = uint(Counter<float>::next());
    output[7] = uint(Counter<int>.next());
    output[8] = uint(Counter<int>.count);
    output[9] = uint(Counter<float>.count);
}
