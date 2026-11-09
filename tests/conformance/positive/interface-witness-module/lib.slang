module lib;
int seed<T>() { return 5; }
public interface I { int read(int input = seed<int>()); }
public struct Item : I
{
    int read(int input = 9) { return input; }
    public int read(float input) { return 20; }
}
public int read<T : I>(T item) { return item.read(); }
public int selected<T : I>(T item) { return item.read(1.0); }
