static int state = 0;
void advance() { state += 1; }

int run() {
    static var unit = advance();
    var copy = unit;
    copy = advance();
    return state;
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = run();
    output[1] = run();
    output[2] = state;
}
