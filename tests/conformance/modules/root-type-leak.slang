import root_type_lib;
typealias Root = float;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = read();
}
