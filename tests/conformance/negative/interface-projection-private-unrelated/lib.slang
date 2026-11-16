module lib;
public interface Read { int read(float input) { return 3; } }
public struct Value : Read {
    internal override int read(int input) { return input + 7; }
    internal int unrelated(float input) { return int(input) + 9; }
}
