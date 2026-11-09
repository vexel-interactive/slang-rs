interface I { int read(); }
int read<T : I>(T item) { return item.missing(); }
[shader("compute")]
[numthreads(1, 1, 1)] void main() { }
