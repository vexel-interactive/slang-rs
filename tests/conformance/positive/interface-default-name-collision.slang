RWStructuredBuffer<uint> output;

interface A_B {
    int C() {
        return 3;
    }
}

interface A {
    int B_C() {
        return 7;
    }
}

struct Item : A_B, A {
};

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    output[0] = item.C();
    output[1] = item.B_C();
}
