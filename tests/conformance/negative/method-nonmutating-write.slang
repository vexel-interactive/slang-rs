struct S { int value; int bad() { value++; return value; } };
[shader("compute")][numthreads(1,1,1)]void main() { S x = {3}; x.bad(); }
