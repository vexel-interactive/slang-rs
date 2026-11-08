float[2] values() { return {1, 2}; }
[shader("compute")][numthreads(1, 1, 1)]void main() { values()[0] = 1; }
