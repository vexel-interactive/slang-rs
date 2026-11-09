interface I { int read(); }
struct Item : I { }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
