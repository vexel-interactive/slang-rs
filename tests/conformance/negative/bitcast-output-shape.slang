[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    double2 input = double2(1);
    uint2 low, high;
    asuint(input, low, high);
}
