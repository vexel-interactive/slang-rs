[shader("compute")]
[numthreads(1, 1, 1)]
void main() { float x = 1.0; float y = 2.0; (x, y) = 3.0; }
