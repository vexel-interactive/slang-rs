// Adds corresponding elements. The application supplies buffers and dispatch size.
StructuredBuffer<float> left;
StructuredBuffer<float> right;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = left[id.x] + right[id.x];
}
