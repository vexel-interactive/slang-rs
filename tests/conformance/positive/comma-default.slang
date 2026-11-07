static float counter = 0.0;
void tick() { counter += 1.0; }
float read(float value = (tick(), counter)) { return value; }
RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { result[0] = (tick(), read()); }
