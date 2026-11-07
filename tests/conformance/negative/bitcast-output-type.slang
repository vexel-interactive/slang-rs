[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float low, high;
    asuint(double(1), low, high);
}
