struct Pair { float x; float y; };
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Pair value = (1, {2.0, 3.0}); }
