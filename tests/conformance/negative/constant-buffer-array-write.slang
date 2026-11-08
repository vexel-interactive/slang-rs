typedef float3 Values[2];
void read(ConstantBuffer<Values> values) { values[1].z = 1; }
[shader("compute")]
[numthreads(1,1,1)]
void main() {}
