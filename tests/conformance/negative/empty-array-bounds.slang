RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1, 1, 1)]
void main() { float values[0]; result[0] = values[0u]; }
