struct Outer<T>
{
    T value = create<T>();
    static int count = 0;
    U copy<U>(U input) { return input; }
    T read() { return copy(value); }
    static U create<U = int>(U input = U()) { return input; }
    static T initial = create<T>();
    [mutating] U update<U>(U input) { count++; value = T(); return input; }
    struct Inner<U>
    {
        T first;
        U second;
        V copy<V>(V input) { return input; }
        T read() { return copy(first); }
    };
};

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Outer<int> first = {2};
    Outer<float> second = {3.0};
    output[0] = uint(first.read());
    output[1] = uint(second.copy(4));
    output[2] = uint(Outer<int>.create());
    output[3] = uint(first.update<int>(5));
    output[4] = uint(Outer<int>.count);
    output[5] = uint(Outer<float>.count);
    Outer<int>.Inner<float> nested = {6, 7.0};
    output[6] = uint(nested.read());
    output[7] = uint(nested.copy<int>(8));
    output[8] = uint(Outer<int>.initial);
    Outer<int> defaulted = Outer<int>();
    output[9] = uint(defaulted.value);
}
