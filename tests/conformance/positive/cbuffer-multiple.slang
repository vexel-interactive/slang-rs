cbuffer Fixed : register(b0) { float fixedValue; };
cbuffer Automatic { float automaticValue; }
uniform float root;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    output[id.x] = fixedValue + automaticValue + root;
}
