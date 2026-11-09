RWStructuredBuffer<uint> output;
struct S
{
    float a, b;
    [ForceInline] __init(float x, float y)
    {
        float local = x;
        a = local;
        b = y;
    }
}
static const S value = S(1.5f, 2.5f);
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    S local = value;
    output[0] = asuint(local.a);
    output[1] = asuint(local.b);
    output[2] = asuint(value.a + value.b);
    output[3] = asuint(1.0f);
}
