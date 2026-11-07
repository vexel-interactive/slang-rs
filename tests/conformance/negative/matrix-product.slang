[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 a = float2x3(1.0);
    float2x4 b = float2x4(1.0);
    float2x4 result = mul(a, b);
}
