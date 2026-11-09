implementing lib;
public struct Member
{
    public struct Inner
    {
        public int value;
        public static int count = 5;
        [ForceInline] public int read() { return value + count; }
    };
};
