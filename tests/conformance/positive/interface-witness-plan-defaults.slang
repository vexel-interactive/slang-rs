T zero<T>() { return T(); }
interface Make<Value> { __init(Value input = zero<Value>()); int read(); }
struct Item<T> : Make<T>
{
    int value;
    __init(T input) { value = 4; }
    __init(float input = 9.0) { value = 8; }
    int read() { return value; }
}
struct Box<T : Make<int>>
{
    T value;
    __init() {}
    int read() { return value.read(); }
}
int automatic<U, T : Make<U>>(U unused) { T item; return item.read(); }
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    output[0] = automatic<int, Item<int>>(0);
    output[1] = automatic<uint, Item<uint>>(0u);
    Box<Item<int>> box;
    output[2] = box.read();
    Item<int> concrete;
    output[3] = concrete.read();
}
