RWStructuredBuffer<uint> output;
interface Read { int read(float input); }
struct Value : Read {
    int read(int input) { return 11; }
    int read(float input, int extra = 7) { return 22 + extra; }
};
int invoke<T : Read>(T item) { return item.read(1); }
interface ConvertRead { int read(float input); }
struct Converted : ConvertRead {
    int read(int input, int extra = 5) { return 13; }
    int read(double input, int extra = 7) { return 17 + extra; }
};
int converted<T : ConvertRead>(T item) { return item.read(1); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Value value;
    output[0] = uint(value.read(1));
    output[1] = uint(invoke(value));
    output[2] = uint(invoke<Value>(value));
    Converted other;
    output[3] = uint(converted(other));
}
