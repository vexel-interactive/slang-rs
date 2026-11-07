[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2 sine;
    float2 cosine;
    sincos(float2(1.0), sine.xx, cosine);
}
