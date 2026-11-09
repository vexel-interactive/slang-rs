RWStructuredBuffer<uint> output;
static int serial = 0;
struct Item
{
    int value;
    [ForceInline] __init() { value = 8; serial += 1; }
};
struct Container
{
    Item item;
    int bias = 3;
    [ForceInline] __init(int value) { bias += value; }
};
struct Grid { Item cells[2]; [ForceInline] __init() { cells[0] = Item(); cells[1] = Item(); } };
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const Item a;
    Item b = {};
    Item c = Item();
    output[0] = uint(a.value + b.value + c.value);
    output[1] = uint(serial);
    Container value = Container(4);
    output[2] = uint(value.item.value);
    output[3] = uint(value.bias);
    output[4] = uint(serial);
    Item array[2] = { Item(), Item() };
    output[5] = uint(array[0].value + array[1].value);
    output[6] = uint(serial);
    Grid grid;
    output[7] = uint(grid.cells[0].value + grid.cells[1].value);
    output[8] = uint(serial);
}
