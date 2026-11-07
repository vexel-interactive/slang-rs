[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const uint low = 0u;
    uint high;
    asuint(double(1), low, high);
}
