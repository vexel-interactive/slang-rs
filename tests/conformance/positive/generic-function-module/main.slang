import lib;
struct Private { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Private value = {3};
    output[0] = uint(relay<Private>(value).value);
    output[1] = uint(copy<int>(5));
}
