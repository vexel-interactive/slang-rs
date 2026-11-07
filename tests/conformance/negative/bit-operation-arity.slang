[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint high = firstbithigh(1u, 2u);
}
