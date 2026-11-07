static float baseValue = 2.0;
struct Material { float3 color = {1.0}; };

float evaluate(Material material = {}, float weights[3] = {1.0, 2.0}, float bias = baseValue)
{
    return material.color.x + weights[0] + weights[1] + weights[2] + bias;
}

RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float baseValue = 9.0;
    result[0] = evaluate() + evaluate({{0.5}}) + baseValue;
}
