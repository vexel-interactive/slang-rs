float read(float x) { return x; }
float read(float x, float y = 3.0) { return x + y; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read(2.0); }
