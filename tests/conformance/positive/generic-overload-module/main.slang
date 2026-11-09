import lib;
struct Private { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Private value = {3};
    output[0] = uint(pick<Private>(value));
    output[1] = uint(resolve<Private>(value));
}
