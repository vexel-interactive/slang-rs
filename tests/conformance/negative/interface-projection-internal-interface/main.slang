module main;
import lib;
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Value value; output[0] = uint(value.read(1.0)); }
