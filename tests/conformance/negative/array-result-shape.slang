float[2] values() { return float[3](); }
[shader("compute")][numthreads(1, 1, 1)]void main() { values(); }
