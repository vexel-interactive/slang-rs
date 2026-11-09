interface I { void read(out int value); }
struct Item : I { void read(inout int value) { value = 4; } }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
