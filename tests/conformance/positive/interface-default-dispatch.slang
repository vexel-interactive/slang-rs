RWStructuredBuffer<uint> output;

interface ZBase {
    int read() {
        return 3;
    }

    int twice() {
        return read() * 2;
    }
}

interface ADerived : ZBase {
    int read() {
        return 7;
    }
}

struct Item : ADerived {
};

int base<T : ZBase>(T value) {
    return value.read();
}

int derived<T : ADerived>(T value) {
    return value.read();
}

int twice<T : ADerived>(T value) {
    return value.twice();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = base(item);
    output[1] = derived(item);
    output[2] = item.read();
    output[3] = twice(item);
    output[4] = item.twice();
}
