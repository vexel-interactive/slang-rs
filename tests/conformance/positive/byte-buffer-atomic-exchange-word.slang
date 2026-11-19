RWByteAddressBuffer data : register(u0);
RWStructuredBuffer<uint> output : register(u1);

static uint trace = 0;
RWByteAddressBuffer receiver() { trace = trace * 10 + 1; return data; }
uint address() { trace = trace * 10 + 2; return 0; }
uint value() { trace = trace * 10 + 3; return 17; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    data.Store(0, 7u);
    receiver().InterlockedExchange(address(), value());
    output[0] = data.Load(0);
    output[1] = trace;
}
