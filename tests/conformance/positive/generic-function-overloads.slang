int arity<T>() { return 1; }
int arity<T, U = int>() { return 2; }
int specific<T>(T value) { return 3; }
int specific<T>(float value) { return 4; }
int defaults<T = int>(T value) { return 5; }
int defaults<T = float>() { return 6; }
struct Box<T> { T value; };
typealias Wrapped<T> = Box<T>;
int read<T>(Wrapped<T> value);
int read<U>(Box<U> value) { return 7; }
int scalar<T>(int32_t value);
int scalar<U>(int value) { return 8; }
typealias Pair<T> = T[2];
int array<T>(Pair<T> value);
int array<U>(U value[1 + 1]) { return 9; }
struct Holder<T> { T value; int evaluate() { return specific<T>(value); } };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(arity<float>());
    output[1] = uint(arity<float, int>());
    output[2] = uint(specific<int>(2));
    output[3] = uint(specific<int>(2.0));
    output[4] = uint(defaults<int>(2));
    output[5] = uint(defaults());
    Box<int> box = {2};
    output[6] = uint(read<int>(box));
    output[7] = uint(scalar<float>(2));
    int pair[2] = {2, 3};
    output[8] = uint(array<int>(pair));
    Holder<int> holder = {2};
    output[9] = uint(holder.evaluate());
}
