[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const int value = 1;
    value = 2;
}
