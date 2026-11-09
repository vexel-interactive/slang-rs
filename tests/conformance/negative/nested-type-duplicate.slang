struct Outer { struct Inner {}; struct Inner {}; };
[shader("compute")][numthreads(1,1,1)]void main() {}
