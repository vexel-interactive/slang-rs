module lib;
public T copy<T>(T value) { return value; }
public struct Ops
{
    public T relay<T>(T value) { return copy(value); }
    public T create<T>(T value = copy(T())) { return value; }
    internal T privateCopy<T>(T value) { return value; }
};
