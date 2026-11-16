module lib;
public interface Read { int read(float input) { return 99; } }
public struct Internal : Read {
    internal override float read(int input) { return float(input) + 2.75; }
}
public struct Public : Read {
    public override float read(int input) { return float(input) + 4.75; }
}
public struct ExactPublic : Read {
    public override float read(float input) { return input + 6.75; }
}
public struct ExactInternal : Read {
    internal override float read(float input) { return input + 8.75; }
}
public struct Other { public int value; }
public struct Unrelated : Read {
    internal override float read(int input) { return float(input) + 10.75; }
    public float read(Other input) { return float(input.value) + 20.75; }
}
int kind(int value) { return 1; }
int kind(float value) { return 2; }
public int insideExact() { ExactInternal value; return kind(value.read(1.0)); }
public int insideAdapted() { Internal value; return kind(value.read(1)); }
public struct ExactWitness : Read {
    internal override int read(float input) { return 13; }
}
