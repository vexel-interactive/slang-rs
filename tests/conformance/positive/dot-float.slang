StructuredBuffer<float4> input;
RWStructuredBuffer<float> output;
float evaluate0(float value)
{
    return dot(value, value);
}
float evaluate1(float1 value)
{
    return dot(value, value);
}
float evaluate2(float2 value)
{
    return dot(value, value);
}
float evaluate3(float3 value)
{
    return dot(value, value);
}
float evaluate4(float4 value)
{
    return dot(value, value);
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(float(input[0].x));
    output[1] = evaluate1(float1(input[0].x));
    output[2] = evaluate2(float2(input[0].xy));
    output[3] = evaluate3(float3(input[0].xyz));
    output[4] = evaluate4(float4(input[0].xyzw));
}
