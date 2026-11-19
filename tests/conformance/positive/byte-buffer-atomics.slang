struct Result {
    int previous;
}

RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
RWStructuredBuffer<int> signedOutput;
groupshared int sharedPrevious;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    uint previous;
    int signedPrevious;
    int2 lanes;
    int slots[2];
    Result result;

    bytes.Store(0, 10u);
    bytes.InterlockedAdd(0, 2u, previous);
    output[0] = previous;
    bytes.InterlockedMin(0, 7u, result.previous);
    output[1] = uint(result.previous);
    bytes.InterlockedMax(0, 9u, lanes.x);
    output[2] = uint(lanes.x);
    bytes.InterlockedAnd(0, 15u, slots[0]);
    output[3] = uint(slots[0]);
    bytes.InterlockedOr(0, 16u, signedPrevious);
    output[4] = uint(signedPrevious);
    bytes.InterlockedXor(0, 3u, signedOutput[0]);
    bytes.InterlockedExchange(0, 31u, sharedPrevious);
    output[5] = uint(sharedPrevious);
    bytes.InterlockedCompareStore(0, 31u, 41u);
    bytes.InterlockedCompareExchange(0, 41u, 51u, previous);
    output[6] = previous;

    bytes.InterlockedAdd(0, true);
    bytes.InterlockedMin(0, 99.5);
    bytes.InterlockedMax(0, 53);
    bytes.InterlockedAnd(0, 255u);
    bytes.InterlockedOr(0, 64u);
    bytes.InterlockedXor(0, 1u);
    output[7] = bytes.Load(0);
    bytes.InterlockedAdd({0}, {{true}});
    bytes.InterlockedMax({{0}}, {1.5});
    bytes.InterlockedAnd({}, {{}});
    bytes.InterlockedOr({uint1(0)}, {int1(1)});
}
