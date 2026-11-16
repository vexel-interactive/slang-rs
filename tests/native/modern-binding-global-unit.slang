static int state = 0;
void advance() { state += 1; }
static let startup = advance();

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var reference = startup;
    output[0] = state;
}
