import lib;
internal struct Item { int value; };
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Wrapped<Item> first = {{3}};
    Box<Item>.Inner<> second = {{5}};
    output[0] = uint(first.read().value);
    output[1] = uint(second.value.value);
}
