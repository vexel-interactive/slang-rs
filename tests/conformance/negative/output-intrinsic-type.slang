[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float exponent;
    float mantissa = frexp(1.0, exponent);
}
