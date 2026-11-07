[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int2x2 input = 1;
    uint2x2 count = countbits(input);
}
