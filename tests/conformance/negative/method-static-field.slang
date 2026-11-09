static int value = 99;
struct S { int value; static int read() { return value; } };
[shader("compute")][numthreads(1,1,1)]void main() { S.read(); }
