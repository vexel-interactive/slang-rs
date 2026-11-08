cbuffer Data { float scale = 2; const int count = 4; };
float Params;
float evaluate(float value = scale) { return value + float(count); }
float shadow(float scale) { return scale + Params; }
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    output[id.x] = evaluate() + shadow(3);
}
