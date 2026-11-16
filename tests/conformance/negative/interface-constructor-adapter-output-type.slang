interface IBuild { __init(out int value); }
struct Value : IBuild {
    int value;
    __init(out float input) {
        input = 2;
        value = 3;
    }
}
T make<T : IBuild>(out int input) { return T(input); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    int input;
    Value value = make<Value>(input);
}
