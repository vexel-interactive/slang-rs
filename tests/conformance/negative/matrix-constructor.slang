[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 a = float2x3(1, 2, 3, 4, 5);
}
