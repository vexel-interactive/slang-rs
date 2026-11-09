struct Outer { static int count = 3; struct Inner {}; };
RWStructuredBuffer<int> output;
[shader("compute")][numthreads(1,1,1)]void main() { output[0] = Outer.Inner.count; }
