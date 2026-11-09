import lib;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main() { output[0] = uint(Shared.value); }
