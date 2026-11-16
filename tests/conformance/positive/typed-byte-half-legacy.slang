ByteAddressBuffer input;
RWByteAddressBuffer output;

[shader("compute")]
[numthreads(2, 1, 1)]
void main(uint3 thread : SV_DispatchThreadID)
{
    uint address = thread.x * 2;
    half value = input.Load<half>(address);
    output.Store<half>(address, value);
}
