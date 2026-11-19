RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    uint1 previous;
    bytes.InterlockedAdd(0, 1u, previous);
    output[0] = previous.x;
}
