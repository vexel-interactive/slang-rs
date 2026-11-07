float read(float value);
float read(float value = 2.0) { return value; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read(); }
