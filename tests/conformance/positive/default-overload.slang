float read(float x, int y = 3) { return x + y; }
float read(int x, int y = 4) { return x + y; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read(2) + read(2.0) + read(2, 5); }
