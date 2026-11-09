struct S { int value; [mutating] void set(int x) { value = x; } };
StructuredBuffer<S> input;
[shader("compute")][numthreads(1,1,1)]void main() { input[0].set(4); }
