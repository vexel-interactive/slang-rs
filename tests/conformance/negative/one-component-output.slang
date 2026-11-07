[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float1 input = 1;
    float sine, cosine;
    sincos(input, sine, cosine);
}
