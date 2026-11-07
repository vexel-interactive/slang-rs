[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint high;
    asuint(double(1), 0u, high);
}
