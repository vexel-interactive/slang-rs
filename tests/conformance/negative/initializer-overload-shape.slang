struct Pair { float x; float y; };
float read(float x) { return x; }
float read(Pair value) { return value.y; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read({1.0, 2.0}); }
