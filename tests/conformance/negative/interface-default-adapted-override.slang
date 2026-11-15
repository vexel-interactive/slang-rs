RWStructuredBuffer<uint> output;

interface I {
    int read(int value) {
        return 3;
    }

    int read(float value) {
        return 7;
    }
}

struct Item : I {
    override int read(int value) {
        return 11;
    }
};

int constrained<T : I>(T value) {
    return value.read(1.0);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = item.read(1);
    output[1] = item.read(1.0);
    output[2] = constrained(item);
}
