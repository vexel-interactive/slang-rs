RWStructuredBuffer<uint> output;

interface ZBase {
    static int read() {
        return 3;
    }

    static int twice() {
        return read() * 2;
    }
}

interface ADerived : ZBase {
    static int read() {
        return 7;
    }
}

struct Item : ADerived {
};

int base<T : ZBase>() {
    return T.read();
}

int derived<T : ADerived>() {
    return T.read();
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = base<Item>();
    output[1] = derived<Item>();
    output[2] = Item.read();
    output[3] = Item.twice();
}
