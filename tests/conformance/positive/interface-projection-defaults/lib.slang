module lib;
internal float seed() { return 3.9; }
public interface Read { int read(float input = seed()) { return 99; } }
public struct Value : Read {
    internal override float read(int input) { return float(input) + 2.75; }
}
