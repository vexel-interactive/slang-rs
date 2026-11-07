StructuredBuffer<uint4> input;
RWStructuredBuffer<int> output;
int evaluate0(bool value)
{
    return dot(value, value);
}
int evaluate1(bool1 value)
{
    return dot(value, value);
}
int evaluate2(bool2 value)
{
    return dot(value, value);
}
int evaluate3(bool3 value)
{
    return dot(value, value);
}
int evaluate4(bool4 value)
{
    return dot(value, value);
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(bool(input[0].x != uint(0)));
    output[1] = evaluate1(bool1(input[0].x != uint1(0)));
    output[2] = evaluate2(bool2(input[0].xy != uint2(0)));
    output[3] = evaluate3(bool3(input[0].xyz != uint3(0)));
    output[4] = evaluate4(bool4(input[0].xyzw != uint4(0)));
}
