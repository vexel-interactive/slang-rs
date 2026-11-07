RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 sine;
    float2x3 cosine;
    sincos(float2x3(1.25), sine, cosine);

    int2x3 exponent;
    float2x3 mantissa = frexp(float2x3(4.0), exponent);

    float2x3 integral;
    float2x3 fraction = modf(float2x3(1.25), integral);
    result[0] = sine[0][0] + cosine[1][2] + mantissa[0][0]
        + float(exponent[1][2]) + fraction[0][0] + integral[1][2];
}
