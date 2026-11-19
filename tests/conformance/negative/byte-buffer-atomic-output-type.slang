RWByteAddressBuffer bytes;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    float previous;
    bytes.InterlockedAdd(0, 1u, previous);
    output[0] = previous;
}
