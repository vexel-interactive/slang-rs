[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 a = float2x3(1.0);
    float value = determinant(a);
}
