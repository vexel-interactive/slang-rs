struct Outer {};
[shader("compute")][numthreads(1,1,1)]void main() { Outer.Inner value; }
