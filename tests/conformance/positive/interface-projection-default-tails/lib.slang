module lib;
public interface Single { int read(float input); }
public interface Empty { int read(); }
public struct PublicOne : Single {
    public float read(float input, int extra = 3) { return input + extra + 0.75; }
}
public struct InternalOne : Single {
    internal float read(float input, int extra = 3) { return input + extra + 0.75; }
}
public struct PublicZero : Empty {
    public float read(int extra = 3) { return float(extra) + 0.75; }
}
public struct InternalZero : Empty {
    internal float read(int extra = 3) { return float(extra) + 0.75; }
}
int kind(int value) { return 1; }
int kind(float value) { return 2; }
public int insidePublicOne() { PublicOne value; return kind(value.read(1.0)); }
public int insideInternalOne() { InternalOne value; return kind(value.read(1.0)); }
public int insidePublicZero() { PublicZero value; return kind(value.read()); }
public int insideInternalZero() { InternalZero value; return kind(value.read()); }
