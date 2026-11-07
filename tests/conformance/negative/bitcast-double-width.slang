[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint3 words = uint3(0);
    double3 values = asdouble(words, words);
}
