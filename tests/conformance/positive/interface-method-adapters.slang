RWStructuredBuffer<uint> output;

interface Read {
    int read(float value);
}
struct Narrow : Read {
    float read(int value) { return float(value) + 0.75; }
};
int read<T : Read>(T value, float input) { return value.read(input); }

interface StaticRead {
    static int read(float value);
}
struct StaticNarrow : StaticRead {
    static float read(int value) { return float(value) + 2.75; }
};
int staticRead<T : StaticRead>(float input) { return T.read(input); }

interface Change {
    [mutating] int change(float input, inout int value, out int copied);
}
struct Counter : Change {
    int total;
    [mutating] float change(int input, inout int value, out int copied) {
        total += input;
        value += input;
        copied = total;
        return float(value) + 0.75;
    }
};
int change<T : Change>(inout T value, float input, inout int state, out int copied) {
    return value.change(input, state, copied);
}

interface GenericRead<A> { A read(float input); }
struct GenericNarrow : GenericRead<int> {
    float read(int input) { return float(input) + 4.75; }
};
int genericRead<T : GenericRead<int>>(T value) { return value.read(3.9); }

interface Defaults {
    int read(float input = 3.9) { return 99; }
}
struct Override : Defaults {
    override float read(int input) { return float(input) + 6.75; }
};
int defaultRead<T : Defaults>(T value) { return value.read(); }

interface StaticReceiver {
    static int read(This value) { return 99; }
}
struct Receiver : StaticReceiver {
    int number;
    override int read() { return number; }
};
int receiverRead<T : StaticReceiver>(T value) { return T.read(value); }

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Narrow narrow;
    output[0] = uint(read(narrow, 3.9));
    output[1] = uint(read<Narrow>(narrow, 5.9));
    output[2] = uint(staticRead<StaticNarrow>(3.9));
    Counter counter = { 1 };
    int state = 2;
    int copied;
    output[3] = uint(change(counter, 3.9, state, copied));
    output[4] = uint(counter.total);
    output[5] = uint(state);
    output[6] = uint(copied);
    GenericNarrow generic;
    output[7] = uint(genericRead(generic));
    Override overridden;
    output[8] = uint(defaultRead(overridden));
    Receiver receiver = { 13 };
    output[9] = uint(receiverRead(receiver));
}
