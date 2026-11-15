RWStructuredBuffer<uint> output;

interface Root {
    int read() {
        return 3;
    }
}

interface Left : Root {
    int read() {
        return 5;
    }
}

interface Right : Root {
}

interface Both : Left, Right {
}

struct Item : Both {
};

int root<T : Root>(T value) {
    return value.read();
}

int left<T : Left>(T value) {
    return value.read();
}

int right<T : Right>(T value) {
    return value.read();
}

int both<T : Both>(T value) {
    return value.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = root(item);
    output[1] = left(item);
    output[2] = right(item);
    output[3] = both(item);
    output[4] = item.read();
}
