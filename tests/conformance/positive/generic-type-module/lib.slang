module lib;
public struct Box<T>
{
    public T value;
    public T read() { return value; }
    public struct Inner<U = T> { public U value; };
    internal struct Hidden<U> { U value; };
};
public typealias Wrapped<T> = Box<T>;
