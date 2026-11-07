StructuredBuffer<int4> input;
RWStructuredBuffer<int> output;
int evaluate0(int value)
{
    return dot(value, value);
}
int evaluate1(int1 value)
{
    return dot(value, value);
}
int evaluate2(int2 value)
{
    return dot(value, value);
}
int evaluate3(int3 value)
{
    return dot(value, value);
}
int evaluate4(int4 value)
{
    return dot(value, value);
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(int(input[0].x));
    output[1] = evaluate1(int1(input[0].x));
    output[2] = evaluate2(int2(input[0].xy));
    output[3] = evaluate3(int3(input[0].xyz));
    output[4] = evaluate4(int4(input[0].xyzw));
}
