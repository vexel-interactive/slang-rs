RWStructuredBuffer<uint> output;
interface Read { int read(float input); }
struct Value : Read {
    int read(int input) { return 11; }
    int read(double input) { return 22; }
};
int invoke<T : Read>(T value) { return value.read(1); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Value value;
    output[0] = uint(value.read(1));
    output[1] = uint(invoke(value));
    output[2] = uint(invoke<Value>(value));
}
