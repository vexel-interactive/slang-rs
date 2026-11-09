interface I { static int read(); }
struct Item : I { int read() { return 4; } }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
