int choose<T>(T value) { return 1; }
int choose<T>(float value) { return 2; }
[shader("compute")][numthreads(1, 1, 1)]
void main() { choose<float>(2.0); }
