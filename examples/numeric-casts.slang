RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    int3 integers = (int3)float3(1.5, 2.5, 3.5);
    bool3 enabled = (bool3)integers;
    float3 weights = (vector<float, 3>)enabled;
    float3x3 source = (float3x3)2.0;
    float2x2 topLeft = (matrix<float, 2, 2>)source;
    int2x2 converted = (int2x2)source;
    float3 flags = (float3)true;
    result[id.x] = weights.x + topLeft[1][1] + float(integers.z)
        + float(converted[0][1]) + flags.y;
}
