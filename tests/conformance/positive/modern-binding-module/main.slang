import library;

int helper() { return 100; }

RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var item = copy(value);
    item.value += 1;
    let immutable = item;
    output[0] = uint(immutable.read());
    output[1] = uint(value.read());
}
