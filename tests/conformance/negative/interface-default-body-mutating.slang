interface I { [mutating] void change(); void read() { change(); } }
[shader("compute")][numthreads(1, 1, 1)] void main() {}
