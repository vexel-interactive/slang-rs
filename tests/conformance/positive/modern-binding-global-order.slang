static var computed = relay();
static var biased = source + 1;
static var source = 4;

int relay() {
    int source = 2;
    return read() + source;
}

int read() { return biased * 2; }

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = uint(computed);
    output[1] = uint(biased);
    output[2] = uint(source);
}
