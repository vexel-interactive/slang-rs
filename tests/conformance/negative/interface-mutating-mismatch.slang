interface I { int read(); }
struct Item : I { [mutating] int read() { return 4; } }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
