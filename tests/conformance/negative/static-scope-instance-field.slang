struct S { int field; };
[shader("compute")][numthreads(1, 1, 1)]
void main() { S s = {1}; int value = s::field; }
