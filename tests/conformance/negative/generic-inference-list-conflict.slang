T pick<T>(T first, T second) { return second; }
int run() { return pick({1, 2}, int2(3, 4)).y; }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main() { output[0] = uint(run()); }
