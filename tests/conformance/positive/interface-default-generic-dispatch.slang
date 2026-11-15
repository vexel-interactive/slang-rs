RWStructuredBuffer<uint> output;

interface Base<Value> {
    Value read(Value value) {
        return value;
    }
}

interface Derived<Value> : Base<Value> {
    Value read(Value value) {
        return Value();
    }
}

struct Item : Derived<int> {
};

int base<T : Base<int>>(T value) {
    return value.read(3);
}

int derived<T : Derived<int>>(T value) {
    return value.read(7);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = base(item);
    output[1] = derived(item);
    output[2] = item.read(7);
}
