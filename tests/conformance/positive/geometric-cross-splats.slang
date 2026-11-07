StructuredBuffer<int3> input;
RWStructuredBuffer<int3> output;
[shader("compute")][numthreads(1, 1, 1)]void main()
{
    int3 value = input[0];
    output[0] = cross(int1(value.x), value);
    output[1] = cross(value.x, value);
    output[2] = int3(cross(half1(value.x), half3(value)));
}
