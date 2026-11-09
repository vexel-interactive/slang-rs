T copy<T>(T value) { return value; }
[shader("compute")][numthreads(1, 1, 1)] void main() { copy<int>(); }
