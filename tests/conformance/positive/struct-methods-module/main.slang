import lib;
RWStructuredBuffer<uint> output;
static int bias = 99;
static int value = 88;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Alias item = Alias(3);
    output[0] = uint(item.read());
    output[1] = uint(item.add(2));
    output[2] = uint(item.read(1));
    output[3] = uint(Alias.fixed());
    output[4] = uint(bias + value);
}
