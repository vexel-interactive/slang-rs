import library;

int extra() { return 100; }

typedef float Secret;

struct Private : IBuild {
    int value;

    __init(Secret input, int tail = 5) { value = int(input) + tail; }
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = make<Value>(2).value;
    output[1] = makeDefault<Value>().value;
    output[2] = make<Private>(3).value;
}
