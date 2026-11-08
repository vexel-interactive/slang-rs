cbuffer Params { float scale; };
cbuffer Other { float scale; };
float evaluate() { return scale; }

[shader("compute")]
[numthreads(1,1,1)]
void main() {}
