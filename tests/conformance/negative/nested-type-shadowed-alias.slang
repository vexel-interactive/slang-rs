struct Outer
{
    typedef int Number;
    struct Inner { int Number; int read() { Number value = 3; return value; } };
};
[shader("compute")][numthreads(1,1,1)]void main() {}
