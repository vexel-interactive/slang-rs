float value : register(b2);
RWStructuredBuffer<float> output : register(u0);

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = value;
}
