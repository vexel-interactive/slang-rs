struct Box<T>
{
    T value;
    static int count = 0;
    __init() { value = T(); count++; }
    __init(T input) { value = input; count++; }
    __init(T first, T second) { value = second; count++; }
    T read() { return value; }
};
struct Outer<T> { Box<T> inner; };
int readDefault(Box<int> value = Box<int>(6, 8)) { return value.read(); }

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Box<int> a;
    Box<int> b = {3};
    Box<int> c = Box<int>(4);
    output[0] = uint(a.read() + b.read() + c.read() + Box<int>.count);
    Outer<int> nested = {{5}};
    output[1] = uint(nested.inner.read());
    output[2] = uint(readDefault());
    output[3] = uint(Box<int>.count);
}
