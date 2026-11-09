struct Outer { struct Item {}; static int Item() { return 1; } };
[shader("compute")][numthreads(1,1,1)]void main() {}
