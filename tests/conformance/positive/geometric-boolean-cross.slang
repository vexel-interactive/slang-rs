StructuredBuffer<uint3> input;
RWStructuredBuffer<int3> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    bool3 flags = input[0] != uint3(0);
    bool scalar = input[0].x != 0u;
    output[0] = cross(flags, flags);
    output[1] = cross(scalar, scalar);
    output[2] = cross(flags, float3(input[0]));
}
