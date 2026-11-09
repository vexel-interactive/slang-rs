module main;
import lib;
struct Item : I
{
    int get(int input = 100) { return input + 4; }
}
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item;
    int offset = 100;
    output[0] = invoke(item);
}
