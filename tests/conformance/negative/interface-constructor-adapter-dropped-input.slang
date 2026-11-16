interface IBuild { __init(int value, int extra); }
struct Value : IBuild {
    int value;
    __init(float input) { value = int(input); }
}
T make<T : IBuild>(int input) { return T(input, 3); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Value value = make<Value>(2); }
