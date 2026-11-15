module direct;

import lib;

RWStructuredBuffer<uint> output;

[numthreads(1, 1, 1)]
void main() {
    Private value;
    output[0] = value.secret();
}
