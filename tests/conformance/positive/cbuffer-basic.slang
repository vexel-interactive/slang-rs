cbuffer Params : register(b2, space1)
{
    float scale;
    float2 values[2];
};
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    output[id.x] = scale + values[1].x;
}
