RWStructuredBuffer<uint> output;
interface Change {
    [mutating] int change(float input, inout int value);
}
struct OutValue : Change {
    int total;
    int change(int input, out int value) { value = input + 2; return input + 4; }
};
struct InValue : Change {
    int total;
    int change(int input, int value) { return value + input; }
};
int invoke<T : Change>(inout T item, inout int value) { return item.change(3.9, value); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    OutValue first = { 9 };
    int state = 1;
    output[0] = uint(invoke(first, state));
    output[1] = uint(state);
    output[2] = uint(first.total);
    InValue second = { 21 };
    state = 10;
    output[3] = uint(invoke(second, state));
    output[4] = uint(state);
    output[5] = uint(second.total);
}
