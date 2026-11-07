StructuredBuffer<int3> input;
RWStructuredBuffer<float3> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    int3 value = input[0];
    half3 normal = half3(0, 1, 0);
    half gap = distance(value, normal);
    half3 mirror = reflect(value, normal);
    half3 bent = refract(value, normal, half(0.5));
    half3 product = cross(value, normal);
    output[0] = float3(mirror + bent + product + gap);
    output[1] = reflect(float3(value), 1.0);
}
