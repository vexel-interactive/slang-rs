module lib;
public interface Left { int read(float input) { return 3; } }
public interface Right { int read(float input) { return 7; } }
public struct Value : Left, Right {
    internal override int read(int input) { return input + 10; }
}
