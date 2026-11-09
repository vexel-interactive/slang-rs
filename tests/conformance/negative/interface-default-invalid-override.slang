interface I { int read() { return 3; } }
struct Item : I { override int read(int input) { return input; } }
[shader("compute")][numthreads(1, 1, 1)] void main() {}
