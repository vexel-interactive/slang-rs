module main;
import lib;
struct Item : IValue
{
    int value;
    __init(int input) { value = input; }
    int read() { return value; }
}
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { Item item = {7}; output[0] = read(item); output[1] = create<Item>(11); }
