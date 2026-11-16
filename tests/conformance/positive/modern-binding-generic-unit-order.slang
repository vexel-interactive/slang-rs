static int state = 0;

void advance() { state = state * 10 + 2; }
int before() { state = state * 10 + 1; return state; }
int after() { state = state * 10 + 3; return state; }

int capture<T>(int first, T unit, int last) {
    var copy = unit;
    return first + last;
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = capture(before(), advance(), after());
    output[1] = state;
}
