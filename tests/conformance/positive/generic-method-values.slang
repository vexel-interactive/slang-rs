struct Ops
{
    T copy<T = int>(T value) { return value; }
    U choose<T, U = T>(T first, U second = U()) { return second; }
    T prototype<T = int>(T value = T());
    U prototype<U>(U renamed) { return renamed; }
    static T zero<T = int>(T value = T()) { return value; }
    void assign<T>(out T result, T value) { result = value; }
    vector<T, 2> shaped<T>(vector<T, 2> value) { return value; }
    T first<T>(T values[2]) { return values[0]; }
    T preferred<T>(T value) { return value; }
    int preferred(int value) { return value + 10; }
};

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Ops ops = {};
    output[0] = uint(ops.copy(2));
    output[1] = uint(ops.copy<float>(3.0));
    output[2] = uint(ops.copy<>(4));
    output[3] = uint(ops.choose<int>(2, 6.0));
    output[4] = uint(ops.choose(2.0));
    output[5] = uint(ops.prototype<>());
    output[6] = uint(Ops.zero());
    output[7] = uint(ops.shaped(int2(7, 8)).y);
    int value = 0;
    ops.assign(value, 9);
    output[8] = uint(value);
    int values[2] = {11, 12};
    output[9] = uint(ops.first(values));
    output[10] = uint(ops.preferred(2));
    output[11] = uint(ops.preferred<int>(2));
}
