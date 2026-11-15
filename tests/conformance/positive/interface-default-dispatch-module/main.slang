module main;

import lib;

struct Item : IDerived {
}

RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Item item;
    int offset = 100;
    output[0] = base(item);
    output[1] = derived(item);
    output[2] = item.read();
    output[3] = local();
}
