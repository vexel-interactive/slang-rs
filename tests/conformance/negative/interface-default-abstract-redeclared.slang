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
};

[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = item.read();
}
