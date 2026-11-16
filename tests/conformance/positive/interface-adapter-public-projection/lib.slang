module lib;
internal float seed() { return 3.9; }
public interface Read {
    int read(float input = seed()) { return 99; }
}
public int invoke<T : Read>(T value) { return value.read(); }
public struct LibraryValue : Read {
    internal override float read(int input) { return float(input) + 2.75; }
    internal float privateOnly(int input) { return float(input); }
};
