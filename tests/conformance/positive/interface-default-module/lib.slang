module lib;
int offset() { return 3; }
public interface I
{
    int get(int input = 2);
    int read() { return get() + offset(); }
}
public int invoke<T : I>(T input) { return input.read(); }
