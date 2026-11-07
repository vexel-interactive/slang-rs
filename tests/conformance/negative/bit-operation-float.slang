[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint count = countbits(1.0);
}
