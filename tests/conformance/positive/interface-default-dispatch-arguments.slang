RWStructuredBuffer<uint> output;

interface Base {
    int read(int value = 3) {
        return value;
    }

    int twice() {
        return read() * 2;
    }
}

interface Derived : Base {
    int read(int value = 7) {
        return value;
    }
}

struct Item : Derived {
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
    output[3] = item.twice();
}
