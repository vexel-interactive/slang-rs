void set(out float value) { value = 1; }
cbuffer Params { float scale; };
void modify() { set(scale); }

[shader("compute")]
[numthreads(1,1,1)]
void main() {}
