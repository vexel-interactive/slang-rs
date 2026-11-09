import lib;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main() { int values[Shared.bias]; output[0] = 1; }
