cbuffer Params { float scale; static float counter = 1; };
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    counter = counter + scale;
    output[id.x] = counter;
}
