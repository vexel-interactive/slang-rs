struct S { int value; int read() { return value; } };
[shader("compute")][numthreads(1,1,1)]void main() { S.read(); }
