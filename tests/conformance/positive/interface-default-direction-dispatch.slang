RWStructuredBuffer<uint> output;

interface Base {
    void change(out int value) {
        value = 3;
    }
}

interface Derived : Base {
    void change(inout int value) {
        value += 7;
    }
}

struct Item : Derived {
};

void base<T : Base>(T item, out int value) {
    item.change(value);
}

void derived<T : Derived>(T item, inout int value) {
    item.change(value);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    int a = 1, b = 1, c = 1;
    base(item, a);
    derived(item, b);
    item.change(c);
    output[0] = a;
    output[1] = b;
    output[2] = c;
}
