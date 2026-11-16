module lib;
internal interface Read { int read(float input) { return 99; } }
public struct Value : Read {
    internal override float read(int input) { return float(input) + 2.75; }
}
