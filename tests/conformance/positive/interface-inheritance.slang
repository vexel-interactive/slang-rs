interface Root { This clone(); }
interface Left : Root { int left(); }
interface Right : Root { int right(); }
interface Both : Left, Right { }
struct Item : Both
{
    int value;
    This clone() { return this; }
    int left() { return value; }
    int right() { return 2; }
}
int read<T : Left & Right>(T item) { return item.clone().left() + item.right(); }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item = {3};
    output[0] = read(item);
    output[1] = read<Item>(item);
}
