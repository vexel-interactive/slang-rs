static var first = second + 1;
static let second = 4;
static var computed = helper();
static const let Count = 3;

int helper() { return first + 2; }

struct State {
    static let bias = 2;
    static var value = Count;
    var member = value + bias;
}

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var value = State();
    var total = 0;
    for (var index = 0; index < Count; index++) {
        let term = index + 1;
        total += term;
    }
    State.value += 1;
    output[0] = uint(first);
    output[1] = uint(second);
    output[2] = uint(computed);
    output[3] = uint(value.member);
    output[4] = uint(total);
    output[5] = uint(State.value);
}
