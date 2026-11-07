StructuredBuffer<uint4> input;
RWStructuredBuffer<uint> output;
uint evaluate0(uint value)
{
    return dot(value, value);
}
uint evaluate1(uint1 value)
{
    return dot(value, value);
}
uint evaluate2(uint2 value)
{
    return dot(value, value);
}
uint evaluate3(uint3 value)
{
    return dot(value, value);
}
uint evaluate4(uint4 value)
{
    return dot(value, value);
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
}
