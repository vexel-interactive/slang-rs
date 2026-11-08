cbuffer Params { float2 value : packoffset(c0.w); };
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = value.x; }
