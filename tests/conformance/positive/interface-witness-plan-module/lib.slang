module lib;
int seed<T>() { return 5; }
public interface Make { __init(int input = seed<int>()); int read(); }
public struct Item : Make
{
    int value;
    __init(int input = 9) { value = input; }
    public __init(float input) { value = 20; }
    int read() { return value; }
}
public int automatic<T : Make>() { int seed = 30; T item; return item.read(); }
public int lists<T : Make>() { T item = {1.0}; return item.read(); }
