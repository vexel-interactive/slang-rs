RWStructuredBuffer<uint> output;
T zero<T>() { return T(); }
int dependency<T>() { T ignored = zero<T>(); return 23; }
interface Read { int read(float input); }
struct Value<T> : Read {
    T field;
    int read(int input, int extra = dependency<int>()) { return input + extra; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { output[0] = 0; }
