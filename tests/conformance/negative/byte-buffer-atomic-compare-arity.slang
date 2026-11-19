RWByteAddressBuffer bytes;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    bytes.InterlockedCompareExchange(0, 1u, 2u);
}
