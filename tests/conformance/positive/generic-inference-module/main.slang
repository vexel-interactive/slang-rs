import lib;
struct Private { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Private value = {3};
    output[0] = uint(relay(value).value);
    output[1] = uint(choose(4));
    output[2] = uint(choose(6.0));
}
