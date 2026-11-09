struct Value { int value; };
T create<T>();
U create<U = Value>() { return U(); }
[shader("compute")][numthreads(1, 1, 1)]
void main() { create(); }
