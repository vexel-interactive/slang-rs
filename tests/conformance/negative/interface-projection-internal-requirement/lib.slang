module lib;
public interface Read { internal int read(float input) { return 3; } }
public struct Value : Read {
    internal override int read(int input) { return input + 7; }
}
