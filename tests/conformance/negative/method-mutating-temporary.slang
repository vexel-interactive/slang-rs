struct S { int value; [mutating] void set(int x) { value = x; } };
[shader("compute")][numthreads(1,1,1)]void main() { S(3).set(4); }
