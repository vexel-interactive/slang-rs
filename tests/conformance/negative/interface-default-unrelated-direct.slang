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

[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = item.read();
}
