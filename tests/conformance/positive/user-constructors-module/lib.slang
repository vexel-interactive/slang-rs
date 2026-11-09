module lib;
static const int bias = 4;
public struct Item
{
    internal int a;
    internal int b = 4;
    [ForceInline] public __init(int x) { a = x + 2; }
    public __init(float x, int amount = bias) { a = int(x) + amount; b = 10; }
};
public int total(Item x) { return x.a + x.b; }
public struct Hidden { internal int value; internal __init(int x){value=x;} };
