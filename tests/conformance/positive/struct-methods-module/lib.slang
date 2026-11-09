module lib;
static const int bias = 4;
static int value = 77;
public struct Item
{
    internal int value;
    [ForceInline] public __init(int x) { set(x); }
    [ForceInline] [mutating] internal void set(int x) { value = x + 2; }
    [ForceInline] public int read(int x = bias) { return value + x; }
    [ForceInline] [mutating] public int add(int x) { value += x; return value; }
    [ForceInline] public static int fixed(int x = bias) { return x + 3; }
};
public typealias Alias = Item;
