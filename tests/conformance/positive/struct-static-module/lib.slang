public struct Shared
{
    public static const int Count = 3;
    internal static int value = 4;
    public static int visible = 5;
    internal static const int bias = 2;
    internal int field = bias;
    [ForceInline] public int read(int extra = bias) { return value + field + extra; }
    [ForceInline] public static void add() { value++; }
    [ForceInline] public int index(int items[bias]) { return items[bias - 1]; }
};
