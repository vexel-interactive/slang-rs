static int trace = 0;
static var first = initializeFirst();
static var unrelated = initializeUnrelated();
static var second = initializeSecond();

int initializeFirst() {
    trace = trace * 10 + 1;
    return second + 1;
}

int initializeUnrelated() {
    trace = trace * 10 + 3;
    return trace;
}

int initializeSecond() {
    trace = trace * 10 + 2;
    return 4;
}

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = uint(trace);
    output[1] = uint(first);
    output[2] = uint(unrelated);
    output[3] = uint(second);
}
