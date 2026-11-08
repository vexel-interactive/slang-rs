cbuffer Params { float scale; };
float evaluate() { return Params.scale; }

[shader("compute")]
[numthreads(1,1,1)]
void main() {}
