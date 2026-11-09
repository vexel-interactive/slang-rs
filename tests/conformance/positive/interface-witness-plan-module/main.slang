module main;
import lib;
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    output[0] = automatic<Item>();
    output[1] = lists<Item>();
}
