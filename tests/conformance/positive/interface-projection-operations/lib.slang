module lib;
public interface GenericRead<A> { A read(float input); }
public struct GenericValue : GenericRead<int> {
    internal float read(int input) { return float(input) + 2.75; }
}
public interface StaticRead { static int read(float input); }
public struct StaticValue : StaticRead {
    internal static float read(int input) { return float(input) + 4.75; }
}
public interface ReceiverRead { static int read(This receiver, float input); }
public struct ReceiverValue : ReceiverRead {
    internal int read(int input) { return input + 6; }
}
public interface Change {
    [mutating] int update(float input, inout uint value, out uint copied);
}
public struct Counter : Change {
    public int total;
    [mutating] internal float update(int input, inout int value, out int copied) {
        total += input;
        value >>= input;
        copied = total;
        return float(input) + 8.75;
    }
}
public interface Base { int read(float input) { return 3; } }
public interface Derived : Base { int read(float input) { return 7; } }
public struct Inherited : Derived {}
