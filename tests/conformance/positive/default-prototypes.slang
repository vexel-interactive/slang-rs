float read(float value = 2.0);
float read(float value = 3.0) { return value; }
float other(float value);
float other(float value = 4.0) { return value; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = read() + other(5.0); }
