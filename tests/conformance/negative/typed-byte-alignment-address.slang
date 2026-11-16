ByteAddressBuffer input;
RWByteAddressBuffer output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output.Store(0, input.LoadAligned<float>(2, 4));
}
