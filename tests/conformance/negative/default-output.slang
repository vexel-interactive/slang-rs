void write(out float value = 1.0) { value = 2.0; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { write(); }
