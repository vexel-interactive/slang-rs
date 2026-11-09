struct S { int value; [ForceInline] int read() { return value; } };
static const int result = S(3).read();
[shader("compute")][numthreads(1,1,1)]void main() {}
