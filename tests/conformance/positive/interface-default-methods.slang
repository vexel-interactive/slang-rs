interface Base
{
    int get(int input = 2);
    int read() { return get(1.0) + 1; }
    int defaulted() { return get(); }
    static int value() { return 100; }
    static int twice() { return value() * 2; }
}
interface Derived : Base {}
struct Item : Derived
{
    int get(int input) { return input + 40; }
    int get(float input) { return 5; }
}
struct Overridden : Base
{
    int get(int input) { return input; }
    override int read() { return 9; }
}
int read<T : Base>(T input) { return input.read(); }

T zero<T>() { return T(); }
interface Generic<T>
{
    T get(T input = zero<T>());
    T read() { return get(); }
    static T fallback() { return zero<T>(); }
}
struct GenericItem : Generic<int>
{
    int get(int input) { return input + 4; }
}

RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Item item;
    Overridden overridden;
    GenericItem genericItem;
    output[0] = read(item);
    output[1] = Item::twice();
    output[2] = item.defaulted();
    output[3] = read(overridden);
    output[4] = genericItem.read();
    output[5] = GenericItem::fallback();
}
