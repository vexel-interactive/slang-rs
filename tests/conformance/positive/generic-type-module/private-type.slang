import lib;
[shader("compute")][numthreads(1, 1, 1)]
void main() { Box<int>.Hidden<int> value; }
