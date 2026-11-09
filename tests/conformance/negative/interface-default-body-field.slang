interface I { int read() { return this.value; } }
struct Item : I { int value; }
[shader("compute")][numthreads(1, 1, 1)] void main() {}
