ConstantBuffer<bool> enabled;
ConstantBuffer<uint1> addresses;
RWByteAddressBuffer data;
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    uint previous;
    data.InterlockedAdd({}, {true}, previous);
    data.InterlockedMax({{addresses}}, {{enabled}}, previous);
    data.InterlockedOr({0}, {bool1(true)}, previous);
    data.InterlockedAdd({0}, {float1(1.5)}, previous);
    data.InterlockedCompareExchange({0}, {1.5}, {int1(2)}, previous);
    output[0] = previous;
}
