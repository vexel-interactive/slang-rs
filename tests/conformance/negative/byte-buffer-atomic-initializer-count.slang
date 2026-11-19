RWByteAddressBuffer data;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    uint previous;
    data.InterlockedAdd({0, 1}, 2, previous);
}
