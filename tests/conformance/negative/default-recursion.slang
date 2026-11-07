float read(float x = read()) { return x; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read(); }
