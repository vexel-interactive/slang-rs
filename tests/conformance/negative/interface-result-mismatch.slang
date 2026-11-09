interface I { int read(); }
struct Other { int value; }
struct Item : I { Other read() { return {4}; } }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
