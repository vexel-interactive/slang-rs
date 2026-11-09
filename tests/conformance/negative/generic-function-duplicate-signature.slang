int choose<T>(int32_t value) { return 1; }
int choose<U>(int value) { return 2; }
[shader("compute")][numthreads(1, 1, 1)]
void main() {}
