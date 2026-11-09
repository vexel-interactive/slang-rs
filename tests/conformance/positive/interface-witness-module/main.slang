module main;
import lib;
RWStructuredBuffer<uint> output;
[shader("compute")] [numthreads(1, 1, 1)]
void main()
{
    Item item;
    output[0] = read(item);
    output[1] = selected(item);
}
