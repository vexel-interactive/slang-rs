int readSize() { return 4; }

[shader("compute")]
[numthreads(readSize(), 1, 1)]
void main() {}
