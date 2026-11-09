import lib;
struct Private { int value; };
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Private value = {3};
    pick<Private, int>(value);
}
