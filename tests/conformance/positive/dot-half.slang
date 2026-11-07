StructuredBuffer<half4> input;
RWStructuredBuffer<half> output;
half evaluate0(half value)
{
    return dot(value, value);
}
half evaluate1(half1 value)
{
    return dot(value, value);
}
half evaluate2(half2 value)
{
    return dot(value, value);
}
half evaluate3(half3 value)
{
    return dot(value, value);
}
half evaluate4(half4 value)
{
    return dot(value, value);
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(half(input[0].x));
    output[1] = evaluate1(half1(input[0].x));
    output[2] = evaluate2(half2(input[0].xy));
    output[3] = evaluate3(half3(input[0].xyz));
    output[4] = evaluate4(half4(input[0].xyzw));
}
