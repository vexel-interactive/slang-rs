struct Empty {};
cbuffer First : register(b3) {};
cbuffer Second { static float storage = 1; Empty empty; };
ConstantBuffer<Empty> third;
cbuffer Full { float value; };
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = value + storage; }
