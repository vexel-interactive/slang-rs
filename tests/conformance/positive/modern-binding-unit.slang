static int state = 0;
void advance() { state += 1; }

struct Item {
    var unit = advance();
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let unit = advance();
    var copy = unit;
    copy = advance();
    output[0] = state;
    var item = Item();
    var fieldCopy = item.unit;
    output[1] = state;
    item.unit = advance();
    output[2] = state;
}
