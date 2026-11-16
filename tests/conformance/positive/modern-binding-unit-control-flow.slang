static int state = 0;
void advance() { state += 1; }
void finish(bool condition) {
    let unit = advance();
    return condition ? unit : advance();
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let unit = true ? advance() : advance();
    let copy = false ? unit : advance();
    output[0] = state;
    finish(true);
    output[1] = state;
    finish(false);
    output[2] = state;
}
