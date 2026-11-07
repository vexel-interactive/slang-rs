float read(float x = 1.0, float y) { return x + y; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
