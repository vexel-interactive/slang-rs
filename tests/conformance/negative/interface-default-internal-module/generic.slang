module internal_generic;

import lib;

RWStructuredBuffer<uint> output;

int read<T : IPrivate>(T value) {
    return value.secret();
}

[numthreads(1, 1, 1)]
void main() {
    Private value;
    output[0] = read(value);
}
