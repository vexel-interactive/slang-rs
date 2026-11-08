void set(out float value) { value = 1; }
void read(ConstantBuffer<float> value) { set(value); }
[shader("compute")]
[numthreads(1,1,1)]
void main() {}
