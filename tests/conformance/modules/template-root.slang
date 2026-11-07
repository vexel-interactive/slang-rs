import template_lib;
RWStructuredBuffer<float3> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = read() + Value(1.0);
}
