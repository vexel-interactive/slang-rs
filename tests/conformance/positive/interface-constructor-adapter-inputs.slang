interface IBuild {
    __init(int value = 4);
}

struct Value : IBuild {
    int value;

    __init(float input, int extra = 3) {
        value = int(input) + extra;
    }
}

T explicitValue<T : IBuild>(int input) { return T(input); }
T omittedValue<T : IBuild>() { return T(); }
T contextualValue<T : IBuild>(int input) {
    T value = { input };
    return value;
}
T automaticValue<T : IBuild>() {
    T value;
    return value;
}
T defaultValue<T : IBuild>(T value = T(6)) { return value; }
T[2] arrayValues<T : IBuild>(int input) {
    T values[2] = { { input }, { input + 1 } };
    return values;
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = explicitValue<Value>(2).value;
    output[1] = omittedValue<Value>().value;
    output[2] = contextualValue<Value>(5).value;
    output[3] = automaticValue<Value>().value;
    output[4] = defaultValue<Value>().value;
    output[5] = Value(2.5, 10).value;
    Value values[2] = arrayValues<Value>(3);
    output[6] = values[0].value + values[1].value;
}
