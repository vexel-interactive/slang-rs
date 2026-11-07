ByteAddressBuffer input;
RWByteAddressBuffer output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint a = input.Load(0u);
    uint2 b = input.Load2(4u);
    uint3 c = input.Load3(12u);
    output.Store(0u, a);
    output.Store2(4u, b);
    output.Store3(12u, c);

    uint previous;
    output.InterlockedAnd(24u, a, previous);
    output.InterlockedOr(28u, a, previous);
    output.InterlockedXor(32u, a, previous);
    output.InterlockedExchange(36u, a, previous);
}
