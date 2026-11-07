RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float zero = {};
    float scalar = {2.0};
    float3 splat = {3.0};
    float3 partial = {1.0, 2.0};
    float3 nested = {{4.0, 5.0}, 6.0};
    float3 constructed = float3({4.0, 5.0}, 6.0);
    float2x2 rows = {1.0, 2.0};
    float2x2 flat = {1.0, 2.0, 3.0};
    float2x2 matrix = float2x2({1.0, 2.0}, {3.0, 4.0});
    result[0] = zero + scalar + splat.z + partial.z + nested.z + constructed.z
        + rows[1][0] + flat[1][1] + matrix[1][1];
}
