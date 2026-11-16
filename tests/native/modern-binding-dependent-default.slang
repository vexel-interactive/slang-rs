struct Item {
    var first = 2;
    var second = first + 3;
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var defaults = Item();
    output[0] = defaults.second;
    var supplied = Item(7);
    output[1] = supplied.second;
    Item contextual = { 11 };
    output[2] = contextual.second;
}
