T copy<T>(T value) { return value + value; }
[shader("compute")][numthreads(1, 1, 1)] void main() {}
