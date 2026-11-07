struct Material
{
    float3 color = {1.0};
    float roughness = 0.5;
};

float sum(float values[3])
{
    return values[0] + values[1] + values[2];
}

Material makeMaterial()
{
    return {{0.2, 0.4, 0.6}, 0.8};
}

RWStructuredBuffer<float4> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Material material = makeMaterial();
    float weights[3] = {1.0, 2.0};
    weights = {3.0};
    material = {{0.1, 0.3, 0.5}};
    result[0] = float4(material.color, material.roughness + sum({2.0}) + sum(weights));
}
