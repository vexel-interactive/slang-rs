RWStructuredBuffer<uint> output;

interface Left {
    int read() {
        return 3;
    }
}

interface Right {
    int read() {
        return 7;
    }
}

struct Item : Left, Right {
};

int left<T : Left>(T value) {
    return value.read();
}

int right<T : Right>(T value) {
    return value.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = left(item);
    output[1] = right(item);
}
