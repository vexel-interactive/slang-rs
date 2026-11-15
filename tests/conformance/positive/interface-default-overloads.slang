RWStructuredBuffer<uint> output;

interface Base {
    int read(int value) {
        return 3;
    }
}

interface Derived : Base {
    int read(float value) {
        return 7;
    }
}

struct Item : Derived {
};

int base<T : Base>(T value) {
    return value.read(1);
}

int derived<T : Derived>(T value) {
    return value.read(1);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = base(item);
    output[1] = derived(item);
    output[2] = item.read(1);
    output[3] = item.read(1.0);
}
