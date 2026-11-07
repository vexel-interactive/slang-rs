RWByteAddressBuffer data;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    data.InterlockedAdd(0u, 1u, 0u);
}
