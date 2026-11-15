RWStructuredBuffer<uint> output;

interface Base {
    int read() {
        return 3;
    }
}

interface Derived : Base {
    int read();
}

struct Item : Derived {
    override int read() {
        return 11;
    }
};

int base<T : Base>(T value) {
    return value.read();
}

int derived<T : Derived>(T value) {
    return value.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = base(item);
    output[1] = derived(item);
    output[2] = item.read();
}
