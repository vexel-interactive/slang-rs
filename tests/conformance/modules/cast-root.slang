import cast_lib;
RWStructuredBuffer<float3> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int3 input = int3(1, 2, 3);
    result[0] = (vector<Element, 3>)input;
}
