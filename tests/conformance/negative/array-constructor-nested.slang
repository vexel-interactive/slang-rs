[shader("compute")]
[numthreads(1, 1, 1)]
void main() { float values[2][2] = float[2][2]({1.0, 2.0, 3.0}, {4.0}); }
