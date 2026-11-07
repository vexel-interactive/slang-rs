RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float2x3 transform = float2x3(float3(1, 2, 3), float3(4, 5, 6));
    float2x3 weights = float2x3(2.0);
    float3x2 transposed = transpose(transform);
    float2x2 product = mul(transform, transposed);
    float2 column = mul(transform, float3(1, 2, 3));
    float3 row = mul(float2(1, 2), transform);
    float2x3 scaled = transform * weights + 1.0;
    scaled[0][1] = 7.0;
    result[id.x] = determinant(product) + column.x + row.y + scaled[0][1];
}
