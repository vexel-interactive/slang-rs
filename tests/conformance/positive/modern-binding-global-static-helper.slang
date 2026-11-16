static var first = read();

int read() {
    static var counter = 4;
    counter += 1;
    return counter;
}

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = uint(first);
    output[1] = uint(read());
}
