ByteAddressBuffer input;
RWByteAddressBuffer output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint byteCount;
    input.GetDimensions(byteCount);
    uint4 values = input.Load4(0u);
    output.Store4(0u, values);

    uint original;
    output.InterlockedAdd(16u, byteCount, original);
    output.InterlockedCompareExchange(20u, 0u, original, original);
}
