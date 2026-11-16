interface IBase<T> {
    __init(T value);
}
interface IDerived<T> : IBase<T> {}

struct Box<T> : IDerived<int> {
    int value;
    T marker;

    __init(float input, int extra = 2) { value = int(input) + extra; }
}
T make<T : IDerived<int>>(int input) { return T(input); }
T makeBase<T : IBase<int>>(int input) { return T(input); }

struct Holder<T : IBase<int>> {
    T value;

    __init(int input) { value = { input }; }
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = int(make<Box<float>>(3).value);
    output[1] = makeBase<Box<int>>(4).value;
    output[2] = Holder<Box<int>>(5).value.value;
}
