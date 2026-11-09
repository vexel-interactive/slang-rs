struct S { static const int value = 3; };
[shader("compute")][numthreads(1, 1, 1)]void main() { S.value++; }
