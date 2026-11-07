StructuredBuffer<float3> input;
RWStructuredBuffer<float3> output;
[shader("compute")][numthreads(1, 1, 1)]void main()
{
    float3 value = input[0];
    float gap = distance(value, {0, 1, 0});
    float3 product = cross({1, 0, 0}, value);
    float3 mirror = reflect(value, {0, 1, 0});
    float3 bent = refract(value, {0, 1, 0}, 0.5);
    float3 face = faceforward(value, {1, 0, 0}, {0, 1, 0});
    output[0] = product + mirror + bent + face + gap;
}
