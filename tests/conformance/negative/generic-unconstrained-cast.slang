struct Box<T> { T read() { return (T)3; } };
[shader("compute")][numthreads(1, 1, 1)]
void main() {}
