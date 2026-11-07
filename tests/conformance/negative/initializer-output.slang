void write(out float value) { value = 1.0; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { write({1.0}); }
