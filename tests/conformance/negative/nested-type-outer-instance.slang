struct Outer { int value; struct Inner { int read() { return value; } }; };
RWStructuredBuffer<int> output;
[shader("compute")][numthreads(1,1,1)]void main() { Outer.Inner item; output[0] = item.read(); }
