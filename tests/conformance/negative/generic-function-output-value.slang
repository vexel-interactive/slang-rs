void assign<T>(out T result, T value) { result = value; }
[shader("compute")][numthreads(1, 1, 1)] void main() { assign<int>(1, 2); }
