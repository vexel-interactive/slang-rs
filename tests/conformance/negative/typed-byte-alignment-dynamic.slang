ByteAddressBuffer input;
RWByteAddressBuffer output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 thread : SV_DispatchThreadID)
{
    output.Store(0, input.LoadAligned<float>(0, thread.x));
}
