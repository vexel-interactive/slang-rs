struct Outer
{
    interface IBuild
    {
        __init(int input);
    }
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
}
