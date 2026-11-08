[shader("compute")][numthreads(1, 1, 1)]
void main() { float values[-(-2.5)] = {1, 2}; }
