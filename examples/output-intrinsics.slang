RWStructuredBuffer<float4> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2 sine;
    float2 cosine;
    sincos(1.0, sine, cosine);

    float2 integral;
    float2 fraction = modf(float2(1.25, 2.5), integral);

    int2 exponent;
    float2 mantissa = frexp(float2(4.0, 8.0), exponent);
    result[0] = float4(sine + cosine, fraction + mantissa + float2(exponent));
}
