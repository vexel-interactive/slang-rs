StructuredBuffer<uint4> input;
RWStructuredBuffer<float> output;
float evaluate0(uint value)
{
    float size = length(value);
    float gap = distance(value, value + 1);
    float unit = normalize(value);
    float mirror = reflect(value, value);
    float bent = refract(value, value, float(0.5));
    float1 face = faceforward(value, value, value);
    return size + gap + unit + mirror + bent + face.x;
}
float evaluate1(uint1 value)
{
    float size = length(value);
    float gap = distance(value, value + 1);
    float1 unit = normalize(value);
    float1 mirror = reflect(value, value);
    float1 bent = refract(value, value, float(0.5));
    float1 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
float evaluate2(uint2 value)
{
    float size = length(value);
    float gap = distance(value, value + 1);
    float2 unit = normalize(value);
    float2 mirror = reflect(value, value);
    float2 bent = refract(value, value, float(0.5));
    float2 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
float evaluate3(uint3 value)
{
    float size = length(value);
    float gap = distance(value, value + 1);
    float3 unit = normalize(value);
    float3 mirror = reflect(value, value);
    float3 bent = refract(value, value, float(0.5));
    float3 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
float evaluate4(uint4 value)
{
    float size = length(value);
    float gap = distance(value, value + 1);
    float4 unit = normalize(value);
    float4 mirror = reflect(value, value);
    float4 bent = refract(value, value, float(0.5));
    float4 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(uint(input[0].x));
    output[1] = evaluate1(uint1(input[0].x));
    output[2] = evaluate2(uint2(input[0].xy));
    output[3] = evaluate3(uint3(input[0].xyz));
    output[4] = evaluate4(uint4(input[0].xyzw));
    output[5] = float(cross(uint3(input[0].xyz), uint3(input[1].xyz)).x);
}
