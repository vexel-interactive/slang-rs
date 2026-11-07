RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x2 integral;
    float2x2 fraction = modf(float2x2(1.25), integral);
    float4 sine;
    float4 cosine;
    sincos({1.0, 2.0}, sine.xy, cosine.zw);
    result[0] = integral[0][0] + fraction[1][1] + sine.x + cosine.z;
}
