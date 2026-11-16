module lib;
internal float interfaceSeed() { return 3.9; }
internal int implementationSeed() { return 17; }
public interface Read { int read(float input = interfaceSeed()); }
public int invoke<T : Read>(T value) { return value.read(); }
public struct LibraryValue : Read {
    internal int read(int input, int extra = implementationSeed()) { return input + extra; }
};
public struct PrototypeValue : Read {
    internal int read(int input, int extra = 19);
    internal int read(int renamed, int renamedExtra) { return renamed + renamedExtra; }
};
