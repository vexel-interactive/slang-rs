groupshared int value;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const int previous = 0;
    InterlockedAdd(value, 1, previous);
}
