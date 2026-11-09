module lib;
__include members;
public struct Outer
{
    internal static const int Bias = 2;
    public typedef int Number;
    internal typedef float Hidden;
    public struct Inner
    {
        public Number value;
        public static int shared = 3;
        public __init(Number x = Bias) { value = x + Bias; }
        [ForceInline] public int read(Number x = Bias) { return value + x; }
        internal int secret() { return 100; }
    };
    internal struct Private { int value; };
};
