struct S { int value; [mutating] void set(int x) { value = x; } };
[shader("compute")][numthreads(1,1,1)]void main() { const S x = {3}; x.set(4); }
