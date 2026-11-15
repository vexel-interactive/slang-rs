RWStructuredBuffer<uint> output;

interface Base {
    int read() {
        return 3;
    }
}

interface Derived : Base {
    float read() {
        return 7;
    }
}

struct Item : Derived {
};

int base<T : Base>(T value) {
    return value.read();
}

float derived<T : Derived>(T value) {
    return value.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = base(item);
    output[1] = uint(derived(item));
    output[2] = uint(item.read());
}
