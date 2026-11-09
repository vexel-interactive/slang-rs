struct S { int read() { return 1; } };
[shader("compute")][numthreads(1, 1, 1)]
void main() { S s = {}; int value = s::read(); }
