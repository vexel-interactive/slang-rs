RWStructuredBuffer<uint> output;
[ForceInline] int adjusted(int x) { return x + 3; }
[ForceInline] uint adjusted(uint x) { return x * 2u; }
float adjusted(float x) { return x + 5.0; }
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = uint(adjusted(2)); output[1] = adjusted(2u); output[2] = asuint(adjusted(5.0)); }
