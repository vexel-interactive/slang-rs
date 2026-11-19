RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
static uint _slang_atomic_ignored_0 = 7;
static uint startup = (bytes.InterlockedExchange(0, 1u), 2u);

struct Packet {
    uint value = (bytes.InterlockedExchange(4, 3u), 4u);
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Packet packet = Packet();
    bytes.InterlockedExchange(8, 5u);
    output[0] = startup + packet.value + _slang_atomic_ignored_0;
}
