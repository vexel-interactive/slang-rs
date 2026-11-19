RWByteAddressBuffer bytes;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    uint previous;
    bytes.InterlockedAdd(0, float1(1.0), previous);
}
