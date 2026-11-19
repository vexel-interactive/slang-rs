RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let previous = 0u;
    bytes.InterlockedExchange(0, 1u, previous);
    output[0] = previous;
}
