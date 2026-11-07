StructuredBuffer<double4> input;
RWStructuredBuffer<double> output;
double evaluate0(double value)
{
    return dot(value, value);
}
double evaluate1(double1 value)
{
    return dot(value, value);
}
double evaluate2(double2 value)
{
    return dot(value, value);
}
double evaluate3(double3 value)
{
    return dot(value, value);
}
double evaluate4(double4 value)
{
    return dot(value, value);
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(double(input[0].x));
    output[1] = evaluate1(double1(input[0].x));
    output[2] = evaluate2(double2(input[0].xy));
    output[3] = evaluate3(double3(input[0].xyz));
    output[4] = evaluate4(double4(input[0].xyzw));
}
