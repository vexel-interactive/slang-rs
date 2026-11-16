interface IBuild { __init(int value); }
struct Value : IBuild {
    int value;
    __init(inout float input) {
        input += 2;
        value = int(input);
    }
}
T make<T : IBuild>(int input) { return T(input); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Value value = make<Value>(2); }
