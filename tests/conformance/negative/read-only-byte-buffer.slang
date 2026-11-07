ByteAddressBuffer data;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    data.Store(0u, 1u);
}
