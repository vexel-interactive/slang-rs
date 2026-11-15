RWStructuredBuffer<uint> output;

interface Key {
    static int key();
}

struct A : Key {
    static int key() {
        return 3;
    }
};

struct B : Key {
    static int key() {
        return 7;
    }
};

interface I<Value : Key> {
    int read() {
        return Value::key();
    }
}

struct Item : I<A>, I<B> {
};

int first<T : I<A>>(T value) {
    return value.read();
}

int second<T : I<B>>(T value) {
    return value.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = first(item);
    output[1] = second(item);
}
