T copy<T>(T value) { return (T)value; }
T relay<T>(T value) { return copy<T>(value); }
T zero<T = int>() { return T(); }
T pick<T, U = T>(T first, U second = U()) { return first; }
void assign<T>(out T result, T value) { result = value; }
void exchange<T>(inout T first, inout T second)
{
    T old = first;
    first = second;
    second = old;
}
T select<T>(T first, T second);
U select<U>(U first, U second) { return second; }
T select<T>(T value) { return value; }
int select(int value) { return value + 1; }
struct Box<T> { T value; T read() { return relay<T>(value); } };
struct Item { int value; };
typealias Integer = int;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    int first = 2, second = 3;
    assign<int>(first, 4);
    exchange<int>(first, second);
    Box<int[2]> array = {{5, 6}};
    Item item = {7};
    output[0] = uint(first);
    output[1] = uint(second);
    output[2] = uint(zero<>());
    output[3] = uint(pick<int>(8));
    output[4] = uint(pick<int, float>(9, 10));
    output[5] = uint(relay<Item>(item).value);
    output[6] = uint(array.read()[1]);
    output[7] = uint(copy<Integer>(11));
    output[8] = uint(copy<float2>(float2(12, 13)).y);
    output[9] = uint(select<int>(14, 15));
    output[10] = uint(select<float>(16));
    output[11] = uint(select(17));
}
