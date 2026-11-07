groupshared int value;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    InterlockedCompareExchange(value, 1, 2);
}
