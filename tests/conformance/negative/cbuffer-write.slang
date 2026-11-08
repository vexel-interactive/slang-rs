cbuffer Params { float2 values[2]; };
void modify() { values[0].x = 1; }

[shader("compute")]
[numthreads(1,1,1)]
void main() {}
