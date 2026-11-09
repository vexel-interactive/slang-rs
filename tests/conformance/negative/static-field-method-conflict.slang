struct S { static int value = 2; int value() { return 1; } };
[shader("compute")][numthreads(1, 1, 1)]void main() {}
