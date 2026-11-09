struct S { int field; static int value = field; };
[shader("compute")][numthreads(1, 1, 1)]void main() {}
