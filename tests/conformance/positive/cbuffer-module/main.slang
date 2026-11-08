import library;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    output[id.x] = scale + offset;
}
