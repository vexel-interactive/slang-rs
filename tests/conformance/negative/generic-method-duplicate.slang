struct Ops
{
    T copy<T>(T value) { return value; }
    U copy<U>(U value) { return value; }
};
