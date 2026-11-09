struct Outer<T>
{
    T value;
    static int count = 0;
    static U copy<U>(U input) { return input; }
    __init<U>(U ignored, T input) { value = copy(input); count++; }
    struct Inner
    {
        T value;
        __init<U>(U ignored, T input) { value = copy(input); }
    };
};
T identity<T>(T input) { return input; }
typealias IntOuter = Outer<int>;
static IntOuter global = IntOuter(float2(1, 2), 11);
struct Holder { IntOuter value; };
IntOuter withDefault(IntOuter input = {2.0, 10}) { return input; }
IntOuter listReturn() { return {2.0, 11}; }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    IntOuter first = IntOuter(2.0, 3);
    Outer<float> second = Outer<float>(int2(1, 2), 4.0);
    output[0] = uint(first.value);
    output[1] = uint(second.value);
    output[2] = uint(Outer<int>.count);
    output[3] = uint(Outer<float>.count);
    output[4] = uint(global.value);
    Outer<int>.Inner inner = Outer<int>.Inner(2.0, 5);
    output[5] = uint(inner.value);
    Holder holder = {{2.0, 6}};
    output[6] = uint(holder.value.value);
    IntOuter array[2] = {{2.0, 7}, {int2(1, 2), 8}};
    output[7] = uint(array[0].value + array[1].value);
    output[8] = uint(identity(IntOuter(2.0, 9)).value);
    output[9] = uint(withDefault().value);
    output[10] = uint(listReturn().value);
}
