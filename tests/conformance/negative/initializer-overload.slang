float read(float x) { return x; }
float read(float2 x) { return x.x; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read({1.0}); }
