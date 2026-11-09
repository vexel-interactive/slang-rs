interface I { int read() { return 3; } }
struct Item : I { int read() { return 7; } }
[shader("compute")][numthreads(1, 1, 1)] void main() {}
