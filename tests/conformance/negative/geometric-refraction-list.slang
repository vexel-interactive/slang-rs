[shader("compute")][numthreads(1, 1, 1)]void main()
{
    float3 value = refract(float3(1), float3(1), {0.5});
}
