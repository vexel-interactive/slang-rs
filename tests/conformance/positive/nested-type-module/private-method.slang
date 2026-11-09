import lib;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Outer.Inner value = Outer.Inner(); output[0] = value.secret(); }
