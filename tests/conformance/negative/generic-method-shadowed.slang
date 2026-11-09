struct Ops
{
    T copy<T>(T value) { return value; }
    int read() { int copy = 2; return copy(3); }
};
