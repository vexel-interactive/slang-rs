RWStructuredBuffer<uint> output;
T zero<T>() { return T(); }
int dependency<T>() { T ignored = zero<T>(); return 23; }
interface Read { int read(float input); }
struct Value : Read {
    int read(int input, int extra = dependency<int>()) { return input + extra; }
};
int invoke<T : Read>(T item) { return item.read(3.9); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Value value;
    output[0] = uint(invoke(value));
}
